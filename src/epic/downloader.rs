//! This is the Epic Downloader module responsible for managing epic apps downloads
//! [Legendary](https://github.com/derrod/legendary) is used to authenticate with epic and download the app owned by the user
//! This module requires you be authenticated in legendary using the [crate::app::epic::legendary::Legendary] `auth` function

use std::process::Stdio;
use std::sync::Arc;

use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use tokio::fs;
use tokio::sync::mpsc::Receiver;

use crate::epic::common::{
    execute_legendary_handle_auth_raw, EPIC_MAX_WRITER_QUEUE_EMPTY_ERROR_COUNT,
};

use crate::types::errors::{EmptyResult, ResultWithError};

use super::config::LegendaryConfig;
use super::legendary::Legendary;

lazy_static! {
    // Progress match regex
    static ref DOWNLOADED_SIZE_REGEX: Regex = Regex::new(r"Downloaded: (\d+\.\d+)").unwrap();
    static ref WRITTEN_SIZE_REGEX: Regex = Regex::new(r"Written: (\d+\.\d+)").unwrap();

    static ref VERIFICATION_PROGRESS_REGEX: Regex = Regex::new(r"Verification progress:.*\((\d+\.?\d*)%").unwrap();
    static ref VERIFYING_LARGE_FILE_REGEX: Regex = Regex::new(r"Verifying large file .* (\d+\.?\d*)% \(\d+\.?\d+?\/(\d+\.?\d+?)").unwrap();

    // Success match regex, this means the download finished or there was nothing to download
    static ref SUCCESS_REGEX: Regex = Regex::new(
        r"(Finished installation process|the game is either already up to date or has not changed)"
    )
    .unwrap();

    // Not authed error match regex, also matches if the user is not connected to the internet
    static ref NOT_AUTHED_ERROR_REGEX: Regex =
        Regex::new(r"(No saved credentials|Login failed!)").unwrap();

    // No internet during download error match regex
    pub static ref NO_INTERNET_ERROR_REGEX: Regex =
        Regex::new(r"Max retries exceeded with url").unwrap();
}

pub struct EpicDownloader {
    pub config: LegendaryConfig,
}

impl EpicDownloader {
    pub fn new(config: LegendaryConfig) -> Self {
        EpicDownloader { config }
    }
}

#[async_trait::async_trait]
pub trait ProviderDownloader {
    async fn download(
        &self,
        session_mutex: Arc<tokio::sync::Mutex<()>>,
        user_id: &str,
        download_options: AppDownloadOption,
    ) -> ResultWithError<Receiver<AppDownloadProgress>>;
}

#[async_trait::async_trait]
impl ProviderDownloader for EpicDownloader {
    /// Triggers a download for this provider
    async fn download(
        &self,
        session_mutex: Arc<tokio::sync::Mutex<()>>,
        app_id: &str,
        download_options: AppDownloadOption,
    ) -> ResultWithError<Receiver<AppDownloadProgress>> {
        if app_id == super::overlay::PROVIDER_APP_ID {
            log::info!("Installing EOS Overlay");
            return super::overlay::install(&self.config, user_id).await;
        }

        let (tx, rc) = tokio::sync::mpsc::channel::<AppDownloadProgress>(128);

        let mut info: Option<LegendaryGameInfo> = None;
        let mut installed_app = Storage::new()
            .get_installed_app(owned_app.id.clone())
            .await?
            .ok_or(ApiError::AppNotFound)?;
        let legendary = Legendary::new(Some(self.config.clone()));

        if installed_app.download_status.stage == DownloadStage::Done {
            let app_info = legendary.info(&owned_app.id).await?;
            if let Some(install) = &app_info.install {
                if install.version == app_info.game.version {
                    return Err(ApiError::AppInstalled.into());
                }

                info!(
                    "Installed app version {} is different than latest app version {}",
                    install.version, app_info.game.version
                );
            }

            info = Some(app_info);
        }

        // Gets the game info and install root
        if info.is_none() {
            info = Some(legendary.info(&owned_app.id).await?);
        }

        if !Downloader::new()
            .is_in_progress(&installed_app.owned_app.id)
            .await
        {
            log::info!("Exiting download early because session cannot be found");
            return Ok(rc);
        }

        let info = info.unwrap();
        let disk_size = info
            .manifest
            .tag_disk_size
            .iter()
            .find(|x| x.tag.is_empty())
            .map(|x| x.size)
            .unwrap_or(info.manifest.disk_size);
        let download_size = info
            .manifest
            .tag_download_size
            .iter()
            .find(|x| x.tag.is_empty())
            .map(|x| x.size)
            .unwrap_or(info.manifest.download_size);

        installed_app.install_config.version = info.game.version;
        installed_app.install_config.download_size = download_size;
        installed_app.install_config.disk_size = disk_size;
        installed_app.install_config.executable = info.manifest.launch_exe;
        "windows".clone_into(&mut installed_app.install_config.platform);

        installed_app = storage.update_installed_app(installed_app).await?;

        // Verify enough space is available and get install disk
        let (install_disk, install_root) =
            verify_space_available(disk_size, &installed_app).await?;
        let install_folder = if installed_app.install_config.install_folder.is_empty() {
            get_app_install_dir(&install_root, &owned_app.name)
        } else {
            installed_app.install_config.install_folder.clone()
        };

        // Make sure apps folder exists
        fs::create_dir_all(get_install_base_folder(&install_root)).await?;

        // Get install folder
        let base_path = get_install_base_folder(&install_root);

        // Spawns the child command as an interactive bash session to launch legendary and install an app
        let game_folder = get_last_segment_of_path(&installed_app.install_config.install_folder);
        let mut arguments: Vec<String> = vec![
            "install".to_owned(),
            app_id.to_string(),
            "--yes".to_owned(),
            "--with-dlcs".to_owned(),
            "--base-path".to_owned(),
            base_path,
            "--game-folder".to_owned(),
            game_folder,
            "--skip-sdl".to_owned(),
        ];

        let custom_shared_memory = std::env::var("CUSTOM_LEGENDARY_INSTALL_SHARED_MEMORY");
        if let Ok(custom_shared_memory) = custom_shared_memory {
            let numeric_value = custom_shared_memory.parse::<u64>();
            if let Ok(numeric_value) = numeric_value {
                if numeric_value > 0 {
                    arguments.push("--max-shared-memory".to_owned());
                    arguments.push(custom_shared_memory);
                }
            }
        }

        // Check if verifying game files
        if download_options.contains(AppDownloadOption::Verify) {
            arguments[0] = "repair".to_owned();
            arguments.push("--repair-and-update".to_owned());
        }

        // Execute legendary command
        let mut reaper = ReaperProcess::new(&self.config.legendary_command);
        reaper.with_identifier(app_id.clone());
        reaper
            .command
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        reaper.execute().await?;

        let stdout = reaper.stdout().await?;
        let stderr = reaper.stderr().await?;

        // Set child for session
        info!("Reaper initialized");
        Downloader::new().assign_reaper(&app_id, reaper).await;

        let cloned_user_id = user_id.to_owned();
        tokio::spawn(async move {
            let rx = stream_process_output(stdout, stderr);

            // Per owned app id lock
            let _lock = session_mutex.lock().await;

            if let Err(err) =
                handle_download(cloned_user_id.clone(), &mut installed_app, rx, tx.clone()).await
            {
                log::error!("Error during epic download: err:{err:?}");
                handle_app_download_error(err, &cloned_user_id, &mut installed_app).await;
            }

            if !tx.is_closed() {
                if let Err(err) = tx.send(installed_app.download_status.clone()).await {
                    log::error!("Error sending tx of download status, err:{err:?}");
                }
            }
        });

        info!("Started download session for for app_id:{}", app_id);

        Ok(rc)
    }
}

#[derive(PartialEq)]
enum DownloadSizeType {
    Disk,
    Download,
}

fn parse_size(line: &str, download_size_type: DownloadSizeType) -> ResultWithError<f32> {
    let compare_str = if download_size_type == DownloadSizeType::Disk {
        "Install size: "
    } else {
        "Download size: "
    };

    if !line.contains(compare_str) {
        return Err("No download size found".into());
    }

    let size = line
        .split(compare_str)
        .last()
        .unwrap_or_default()
        .split(' ')
        .next()
        .unwrap_or_default();

    Ok(size.parse()?)
}
