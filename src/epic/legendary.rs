//! This is the legendary module responsible for common functionality within legendary, such as getting status and authentication
use crate::{
    constants::LIBRARY_PROVIDER_ID,
    epic::{
        common::{
            check_invalid_legendary_response, execute_legendary_handle_auth_json, parse_size,
            require_legendary, EPIC_MAX_WRITER_QUEUE_EMPTY_ERROR_COUNT,
        },
        models::{
            DownloadSizeType, EpicStatusResponse, LegendaryGameInfo, LegendaryGameListInfo,
            LegendaryInstalledGameResponse,
        },
    },
    plugin::errors::PluginError,
    types::{
        app::{
            AppDownloadProgress, AppEulaInformation, InstallOption, LaunchOption, LaunchType,
            Platform,
        },
        cloud_sync::CloudSyncOperation,
        results::{EmptyResult, ResultWithError},
    },
    utils::{
        child::{command_sync, stream_process_output, OutputLine},
        reaper::ReaperProcess,
        strings::regex_capture,
        system::{bytes_to_megabytes, megabytes_to_bytes},
    },
};
use chrono::NaiveDateTime;
use log::info;
use parking_lot::Mutex;
use regex::Regex;
use std::process::Stdio;
use std::time::Duration;
use tokio::{process::Command, sync::mpsc::Receiver, time::Instant};

use super::{
    common::execute_legendary_handle_auth_raw,
    config::LegendaryConfig,
    models::{LegendaryEula, LegendaryGameManifestPrerequisites, LegendaryUplayStatus},
};

pub fn get_wine_prefix_path(app_id: &str) -> ResultWithError<String> {
    let prefix_path = format!("/test/{:?}", app_id.to_owned());
    Ok(prefix_path.to_string())
}

lazy_static::lazy_static! {
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

lazy_static::lazy_static! {
    /// Whether owned apps are being fetched
    pub static ref FETCHING_APPS: Mutex<bool> = Mutex::new(false);
    static ref SAVE_DATES_REGEX: Regex = Regex::new(r"save date: (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})").unwrap();
}

#[derive(Clone)]
pub struct Legendary {
    config: LegendaryConfig,
}

impl Legendary {
    pub fn new(config: Option<LegendaryConfig>) -> Self {
        Self {
            config: config.unwrap_or_else(LegendaryConfig::new),
        }
    }

    /// Authorize user to legendary using a webview
    pub async fn auth(&self, authorization_code: Option<String>) -> EmptyResult {
        require_legendary(&self.config.legendary_command).await?;

        // Delete existing credentials
        self.logout().await?;

        // Get exchange token from webview or request
        let Some(code) = authorization_code else {
            log::error!("No authorization code submitted for epic login");
            return Err(PluginError::AuthenticationError.into());
        };

        let (_, res) = execute_legendary_handle_auth_raw(
            self.config.legendary_command.clone(),
            ["auth", "--code", code.as_str()],
        )
        .await?;
        let username = res
            .split("Successfully logged in as")
            .last()
            .unwrap_or_default()
            .split('"')
            .nth(1)
            .unwrap_or_default();

        info!("Authorized legendary for username:{}", username);

        Ok(())
    }

    /// Closes the epic client and removes the saved user's epic credentials
    pub async fn logout(&self) -> EmptyResult {
        log::info!("Logging out of legendary");
        execute_legendary_handle_auth_raw(
            self.config.legendary_command.clone(),
            ["auth", "--delete"],
        )
        .await?;
        Ok(())
    }

    /// Directly performs the sync
    pub async fn perform_cloud_sync(
        &self,
        app_id: &str,
        operation: CloudSyncOperation,
        conflict_resolution: bool,
    ) -> EmptyResult {
        info!(
            "Legendary cloud sync operation:{:?} called for provider_app_id:{}",
            operation, app_id
        );

        let wine_prefix_path = get_wine_prefix_path(app_id)?;
        let mut command = Command::new(&self.config.legendary_command);
        let mut command = command
            .env("WINEPREFIX", wine_prefix_path)
            .args(["sync-saves", app_id, "--accept-path", "-y"])
            .stderr(Stdio::piped());

        match (&operation, conflict_resolution) {
            (CloudSyncOperation::Download, false) => command = command.arg("--skip-upload"),
            (CloudSyncOperation::Upload, false) => command = command.arg("--skip-download"),
            (CloudSyncOperation::Download, true) => command = command.arg("--force-download"),
            (CloudSyncOperation::Upload, true) => command = command.arg("--force-upload"),
        }

        let output = command.output().await?;
        let output = String::from_utf8_lossy(&output.stderr);

        log::debug!("Cloud sync legendary output: {output}");

        if operation == CloudSyncOperation::Download {
            tokio::spawn(command_sync());
        }

        if operation == CloudSyncOperation::Upload
            && output.contains("Save game downloading is disabled, skipping")
        {
            log::error!("Conflict detected when uploading epic game cloud save for provider_app_id:{app_id}");
        } else if operation == CloudSyncOperation::Download
            && output.contains("Save game uploading is disabled, skipping")
        {
            log::error!("Conflict detected when downloading epic game cloud save for provider_app_id:{app_id}");
        }

        if output.contains("is in conflict") {
            let mut captures = SAVE_DATES_REGEX.captures_iter(&output);
            let cloud = captures.next();
            let local = captures.next();

            log::debug!("Cloud: {:?} Local: {:?}", cloud, local);

            match (cloud, local) {
                (Some(cloud), Some(local)) => {
                    let format = "%Y-%m-%d %H:%M:%S";
                    let cloud_date = NaiveDateTime::parse_from_str(&cloud[1], format)?.and_utc();
                    let local_date = NaiveDateTime::parse_from_str(&local[1], format)?.and_utc();
                    log::info!(
                        "TODO: Send {:?} and {:?} back to playserve",
                        cloud_date,
                        local_date
                    );
                }
                _ => log::error!("Unable to find cloud and local timestamps in legendary output"),
            }
        }

        Ok(())
    }

    /// Gets the legendary client status
    pub async fn status(&self) -> ResultWithError<EpicStatusResponse> {
        require_legendary(&self.config.legendary_command).await?;
        let mut response: EpicStatusResponse = execute_legendary_handle_auth_json(
            self.config.legendary_command.clone(),
            ["status", "--json"],
        )
        .await?;
        if response.account == "<not logged in>" {
            response.account = String::new();
        }
        Ok(response)
    }

    /// Gets the info about a certain app using the legendary client
    pub async fn info(&self, app_id: &str) -> ResultWithError<LegendaryGameInfo> {
        info!("Legendary game info called for app:{}", app_id);

        let mut reaper = ReaperProcess::new(&self.config.legendary_command);
        reaper.command.args(["info", app_id, "--json"]);
        let response = reaper
            .execute_with_output_json(Some(check_invalid_legendary_response))
            .await?;

        Ok(response)
    }

    /// Return launch options for a game
    pub async fn get_launch_options(&self, app_id: String) -> ResultWithError<Vec<LaunchOption>> {
        let game_info = self.info(&app_id).await?;
        let mut launch_options: Vec<LaunchOption> = Vec::new();
        launch_options.push(LaunchOption {
            description: "Launch".to_string(),
            executable: game_info.manifest.launch_exe.clone(),
            arguments: game_info.manifest.launch_command.clone(),
            working_directory: "".to_string(),
            environment: [].to_vec(),
            launch_type: LaunchType::Game,
            hardware_tags: [].to_vec(),
        });

        let _: Vec<_> = game_info
            .game
            .launch_options
            .iter()
            .map(|launch_option| {
                launch_options.push(LaunchOption {
                    description: launch_option.name.clone(),
                    executable: game_info.manifest.launch_exe.clone(),
                    arguments: launch_option.parameters.clone(),
                    working_directory: "".to_string(),
                    environment: [].to_vec(),
                    launch_type: LaunchType::Game,
                    hardware_tags: [].to_vec(),
                })
            })
            .collect();
        Ok(launch_options)
    }

    /// Return prerequisites for a game
    pub async fn get_prerequisites(
        &self,
        app_id: &str,
    ) -> ResultWithError<Option<LegendaryGameManifestPrerequisites>> {
        let game_info = self.info(app_id).await?;
        Ok(game_info.manifest.prerequisites)
    }

    /// Download a game
    pub async fn install(
        &self,
        app_id: String,
        base_path: String,
        options: InstallOption,
    ) -> ResultWithError<Receiver<AppDownloadProgress>> {
        let game_info: LegendaryGameInfo = self.info(&app_id).await.unwrap();
        let mut arguments: Vec<String> = vec![
            "install".to_owned(),
            app_id.to_string(),
            "--yes".to_owned(),
            "--with-dlcs".to_owned(),
            "--skip-sdl".to_owned(),
        ];

        // base_path is optional in the context of updates
        if !base_path.is_empty() {
            arguments.push("--base-path".to_owned());
            arguments.push(base_path);
        };
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

        // Check if verifying game files // FIXME
        if options.verify == Some(true) {
            arguments[0] = "repair".to_owned();
            arguments.push("--repair-and-update".to_owned());
        }
        // Execute legendary command
        let mut reaper = ReaperProcess::new(&self.config.legendary_command);
        reaper.with_identifier(app_id.to_string());
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

        tokio::spawn(async move {
            let rx = stream_process_output(stdout, stderr);

            if let Err(err) = handle_download(&game_info, rx).await {
                log::error!("Download failed: {}", err);
                // TODO Send "install failed" signal
                return Err(PluginError::DownloadFailed);
            }
            // TODO Send "install_completed" signal
            Ok(())
        });
        let (_, rc) = tokio::sync::mpsc::channel::<AppDownloadProgress>(128);
        Ok(rc)
    }

    /// Imports a game into legendary
    pub async fn import(&self, app_id: &str, install_folder: &str) -> EmptyResult {
        info!("Legendary import game called for app:{}", app_id);

        let mut reaper = ReaperProcess::new(&self.config.legendary_command);
        reaper.command.args([
            "import",
            app_id,
            install_folder,
            "--with-dlcs",
            "--disable-check",
        ]);
        let (status, _, stderr) = reaper
            .execute_with_output(Some(check_invalid_legendary_response))
            .await?;

        if status != 0
            || (!stderr.contains("has been imported")
                && !stderr.contains("Game is already installed"))
        {
            log::error!("Error occurred when importing app, status:{status}, stderr:{stderr:?}");
            return Err("Error while importing Epic app".into());
        }

        Ok(())
    }

    pub async fn get_installed_apps(&self) -> ResultWithError<Vec<LegendaryInstalledGameResponse>> {
        let mut reaper = ReaperProcess::new(&self.config.legendary_command);
        reaper.command.args(["list-installed", "--json"]);
        let (status, stdout, stderr) = reaper
            .execute_with_output(Some(check_invalid_legendary_response))
            .await?;
        if status != 0 {
            log::error!(
                "Error occurred when listing installed apps, status:{status}, stderr:{stderr:?}"
            );
            return Err("Error while listing Epic apps".into());
        }
        // log::info!("{}", stdout);
        //Parse JSON output
        let installed_apps: Vec<LegendaryInstalledGameResponse> = serde_json::from_str(&stdout)?;
        Ok(installed_apps)
    }

    pub async fn get_apps(&self) -> ResultWithError<Vec<LegendaryGameListInfo>> {
        log::info!("Fetching owned apps for Epic");
        let response: Vec<LegendaryGameListInfo> = execute_legendary_handle_auth_json(
            self.config.legendary_command.clone(),
            ["list", "--json"],
        )
        .await?;
        Ok(response.into_iter().collect::<Vec<LegendaryGameListInfo>>())
    }

    pub fn get_app_versions(&self, apps: Vec<LegendaryGameListInfo>) -> Vec<(String, String)> {
        let mut app_versions = Vec::<(String, String)>::new();
        let _ = apps
            .iter()
            .map(|game_info| {
                if let Some(asset_info) = game_info.asset_infos.get(&Platform::Windows.to_string())
                {
                    app_versions
                        .push((game_info.app_name.clone(), asset_info.build_version.clone()));
                }
            })
            .collect::<Vec<()>>();
        app_versions
    }

    pub async fn get_game_info(
        &self,
        app_id: &str,
    ) -> ResultWithError<Option<LegendaryInstalledGameResponse>> {
        match self.get_installed_apps().await {
            Ok(apps) => {
                for app in apps {
                    if app.app_name == app_id {
                        return Ok(Some(app));
                    }
                }
            }
            Err(e) => log::error!("Failed to get installed apps: {}", e),
        }
        Ok(None)
    }

    /// Move an app to another drive
    pub async fn move_app_to_drive(&self, app_id: &str, dest_path: &str) -> EmptyResult {
        execute_legendary_handle_auth_raw(
            self.config.legendary_command.clone(),
            ["move", app_id, dest_path, "--skip-move"],
        )
        .await?;
        Ok(())
    }

    // Returns only unaccepted EULAs fetched from Epic API
    pub async fn get_eulas(&self, app_id: &str) -> ResultWithError<Vec<AppEulaInformation>> {
        let response = execute_legendary_handle_auth_json(
            self.config.legendary_command.clone(),
            ["eula", app_id, "--skip-epic", "--json"],
        )
        .await;
        if let Err(err) = response {
            log::warn!("Legendary eula error");
            log::debug!("Legendary eula output: {:?}", err);
            return Err(err);
        }
        let response: Vec<LegendaryEula> = response?;
        let eulas: Vec<AppEulaInformation> = response
            .into_iter()
            .map(|e| AppEulaInformation {
                id: e.key,
                name: e.title,
                version: e.version,
                url: e.url,
                body: e.body,
                country: "".to_string(),
                language: e.locale,
                owned_app_id: format!("{}_{}", LIBRARY_PROVIDER_ID, app_id),
            })
            .collect();

        Ok(eulas)
    }

    /// Uninstall an app
    pub async fn uninstall(&self, app_id: &str) -> EmptyResult {
        info!("Running legendary uninstall for app:{}", app_id);
        execute_legendary_handle_auth_raw(
            self.config.legendary_command.clone(),
            ["uninstall", app_id, "--yes"],
        )
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    /// Retrieves the download size of this owned app
    async fn get_download_size(&self, app_id: &str) -> ResultWithError<u64> {
        let info = self.info(app_id).await?;
        let download_size = info
            .manifest
            .tag_download_size
            .iter()
            .find(|x| x.tag.is_empty())
            .map(|x| x.size)
            .unwrap_or(info.manifest.download_size);

        Ok(download_size)
    }

    #[allow(dead_code)]
    /// Gets a summary of activated games on Ubisoft as long as the account is linked
    /// This allows us to run actual activation when it's necessary
    pub async fn uplay_link_status(&self) -> ResultWithError<LegendaryUplayStatus> {
        let (stdout, stderr) = execute_legendary_handle_auth_raw(
            self.config.legendary_command.clone(),
            ["activate", "--uplay", "--summary", "--json"],
        )
        .await?;

        if stderr.contains("No linked ubisoft account found!") {
            return Err("Ubisoft Connect account not linked with Epic Games".into());
        }
        let link_summary = serde_json::from_str(&stdout)?;
        Ok(link_summary)
    }

    #[allow(dead_code)]
    /// Activates Ubisoft games on conntected Ubisoft account
    /// This lets Uplay start the game properly
    pub async fn uplay_activate(&self) -> EmptyResult {
        execute_legendary_handle_auth_raw(
            self.config.legendary_command.clone(),
            ["activate", "--uplay", "-y"],
        )
        .await?;
        Ok(())
    }
}

/// Handles the download thread when download is in progress
async fn handle_download(
    game_info: &LegendaryGameInfo,
    mut rx: Receiver<OutputLine>,
) -> EmptyResult {
    let app_id = game_info.game.app_name.clone();
    info!(
        "Starting new thread to read output from epic download app session for app:{}",
        app_id
    );

    let mut current_progress: f32 = -1.0;
    let mut no_internet_error_count: Option<Instant> = None;

    let mut current_verify_progress: f32 = 0.0;
    let mut current_verify_progress_with_large_file: f32 = 0.0;

    let mut total_size = bytes_to_megabytes(game_info.get_download_size());
    let mut remaining_size = total_size;

    // If we receive this warning in the logs 2 times without any progress, we should retry download
    let mut empty_queue_warning_count = 0;

    while let Some(line) = rx.recv().await {
        let line = match line {
            OutputLine::Stdout(s) | OutputLine::Stderr(s) => s,
        };

        log::debug!("Epicdownloader: {}", line);

        if let Ok(size) = parse_size(&line, DownloadSizeType::Disk) {
            let disk_size = bytes_to_megabytes(game_info.get_disk_size());

            if size > disk_size {
                // installed_app.install_config.disk_size = megabytes_to_bytes(size);
                // Emit download progress
            }
        }

        if let Ok(size) = parse_size(&line, DownloadSizeType::Download) {
            remaining_size = size;

            if remaining_size > total_size {
                total_size = remaining_size;
            }
        }

        // Only trigger queue errors if we are downloading the game properly
        if line.contains("WARNING: Writer queue empty!") && no_internet_error_count.is_none() {
            empty_queue_warning_count += 1;

            if empty_queue_warning_count >= EPIC_MAX_WRITER_QUEUE_EMPTY_ERROR_COUNT {
                log::error!(
                    "Legendary not progressing download for app_id:{}, retrying",
                    app_id
                );
                break;
            }
        }

        if line.contains("No write access to") {
            return Err(line.into());
        }

        // Happens when verifying game files with repair functionality
        if let Some(captures) = VERIFICATION_PROGRESS_REGEX.captures(&line) {
            let Some(percentage) = captures.get(1).and_then(|x| x.as_str().parse::<f32>().ok())
            else {
                continue;
            };

            if percentage != current_verify_progress {
                current_verify_progress = percentage;
                log::info!(
                    "Legendary verification progress for app_id:{}, current: {:.1}%",
                    app_id,
                    current_verify_progress
                );
                // TODO emit install_progressed with Verify stage
            }

            continue;
        }

        // The above output stops and this starts when it starts verifying a large file, so we just add the current_progress to this percentage
        if let Some(captures) = VERIFYING_LARGE_FILE_REGEX.captures(&line) {
            let Some(percentage) = captures.get(1).and_then(|x| x.as_str().parse::<f32>().ok())
            else {
                continue;
            };
            let Some(file_size_mb) = captures.get(2) else {
                continue;
            };
            let file_size_bytes = megabytes_to_bytes(file_size_mb.as_str().parse().unwrap_or(0.0));
            let disk_size = 0; //Need to get the total disk size here
            let weight = if disk_size == 0 {
                0.0
            } else {
                file_size_bytes as f32 / disk_size as f32
            };

            let progress = current_verify_progress + (weight * percentage);
            if progress != current_verify_progress_with_large_file {
                current_verify_progress_with_large_file = progress;
                log::info!(
                    "Legendary verification progress for app_id:{}, current: {:.1}%",
                    app_id,
                    current_verify_progress_with_large_file
                );
                // TODO emit install_progressed with Verify stage
            }
            continue;
        }

        let capture = regex_capture(DOWNLOADED_SIZE_REGEX.to_owned(), line.clone());
        let capture = capture.split(' ').last().unwrap_or_default();
        let downloaded_size = capture.parse::<f32>();
        // let capture = regex_capture(WRITTEN_SIZE_REGEX.to_owned(), line.clone());
        // let capture = capture.split(' ').last().unwrap_or_default();
        // let written_size = capture.parse::<f32>().map_or(0, megabytes_to_bytes);

        if let Ok(downloaded_size) = downloaded_size {
            no_internet_error_count = None;

            let mut progress =
                (((total_size - remaining_size) + downloaded_size) / total_size).min(1.0) * 100.0;
            progress = progress.clamp(0.0, 100.0);

            if progress != current_progress {
                empty_queue_warning_count = 0;
                current_progress = progress;

                log::info!("Verify {}: {}%", app_id, current_progress);
                // TODO emit install_progressed
                continue;
            }
        }

        if SUCCESS_REGEX.is_match(&line) {
            log::info!("Finished downloading the app for app {}", app_id);
            break;
        }

        if NOT_AUTHED_ERROR_REGEX.is_match(&line) {
            log::error!("User not authorized when downloading app {}", app_id);
            return Err(PluginError::NotLoggedIn.into());
        }

        if NO_INTERNET_ERROR_REGEX.is_match(&line) {
            match no_internet_error_count {
                None => no_internet_error_count = Some(Instant::now()),
                Some(instant) => {
                    // Give it 10 seconds to reconnect to the network
                    // Also wait for legendary to potentially restart the workers
                    if instant.elapsed() > Duration::from_secs(10) {
                        log::error!("No internet when downloading app for app_id:{}", app_id);
                        return Err(PluginError::NetworkRequired.into());
                    }
                }
            }
        }
    }

    Ok(())
}
