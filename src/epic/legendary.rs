//! This is the legendary module responsible for common functionality within legendary, such as getting status and authentication
use crate::{
    constants::LIBRARY_PROVIDER_ID,
    epic::common::{
        check_invalid_legendary_response, execute_legendary_handle_auth_json, get_app_install_dir,
        require_legendary,
    },
    plugin::errors::PluginError,
    types::{
        app::{AppEulaInformation, EpicStatusResponse},
        cloud_sync::CloudSyncOperation,
        epic::{LegendaryGameInfo, LegendaryInstalledGameResponse},
        errors::{EmptyResult, ResultWithError},
    },
    utils::{child::command_sync, reaper::ReaperProcess, system::move_folder_with_progress},
};
use chrono::NaiveDateTime;
use futures::StreamExt;
use log::info;
use parking_lot::Mutex;
use regex::Regex;
use std::process::Stdio;
use tokio::{fs, process::Command};

use super::{
    common::execute_legendary_handle_auth_raw, config::LegendaryConfig, models::LegendaryEula,
};

pub fn get_wine_prefix_path(app_id: &str) -> ResultWithError<String> {
    let prefix_path = format!("/test/{:?}", app_id.to_owned());
    Ok(prefix_path.to_string())
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

    /// Imports a game into legendary
    pub async fn import(&self, app_provider_id: &str, install_folder: &str) -> EmptyResult {
        info!(
            "Legendary import game called for provider_id:{}",
            app_provider_id
        );

        let mut reaper = ReaperProcess::new(&self.config.legendary_command);
        reaper.command.args([
            "import",
            app_provider_id,
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

        //Parse JSON output
        let installed_apps: Vec<LegendaryInstalledGameResponse> = serde_json::from_str(&stdout)?;
        Ok(installed_apps)
    }

    pub async fn get_game_info(
        &self,
        app_id: &str,
    ) -> ResultWithError<Option<LegendaryInstalledGameResponse>> {
        for app in self.get_installed_apps().await.unwrap() {
            if app.app_name == app_id {
                return Ok(Some(app));
            }
        }
        Ok(None)
    }

    /// Move an app to another drive
    pub async fn move_app_to_drive(&self, app_id: &str, drive_name: &str) -> EmptyResult {
        if let Some(game_info) = self.get_game_info(app_id).await? {
            let current_folder_name = game_info.install_path.split('/').last().unwrap();
            let new_game_folder_path = get_app_install_dir(drive_name, current_folder_name);

            // use util to move game folder since legendary move doesn't support moving to a different drive
            fs::remove_dir_all(&new_game_folder_path)
                .await
                .unwrap_or_default();

            let mut progress =
                move_folder_with_progress(&game_info.install_path, &new_game_folder_path).await;

            while let Some(progress) = progress.next().await {
                let progress = progress?;
                self.notify_move_progress(progress).await;
            }
            execute_legendary_handle_auth_raw(
                self.config.legendary_command.clone(),
                ["move", app_id, &new_game_folder_path, "--skip-move"],
            )
            .await?;
            Ok(())
        } else {
            Err("Game not installed".into())
        }
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

    pub async fn notify_move_progress(&self, progress: u8) {
        log::info!("{:?}", progress);
        // if let Some(manager) = dbus::get_manager_interface().await {
        //     if let Err(err) = manager
        //         .on_drive_added(
        //             manager.signal_emitter(),
        //             PluginDriveInfo::from_disk_info(info),
        //         )
        //         .await
        //     {
        //         log::error!("Error emitting manager signal for new drive added, err:{err:?}");
        //     }
        // }
    }
}
