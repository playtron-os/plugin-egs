use crate::constants::LIBRARY_PROVIDER_ID;
use crate::epic::overlay;
use crate::epic::{
    common::require_legendary, config::LegendaryConfig, legendary::Legendary,
    models::LegendaryGameListInfo,
};

use crate::plugin::library_provider::LibraryProviderSignals;
use crate::types::app::{
    AppEulaInformation, AppType, InstallOption, InstallScript, InstalledApp, ItemMetadata,
    LaunchOption, PlaytronAppType, PlaytronImage, PlaytronProvider, ProviderItem, Registry,
    RegistryEntry, RequirementOSConfig, RunProcessParams,
};
use crate::types::cloud_sync::CloudSyncOperation;
use crate::types::results::{EmptyResult, ResultWithError};
use crate::utils::system::move_folder_with_progress;
use futures::StreamExt;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio_util::sync::CancellationToken;
use zbus::fdo;
use zbus::object_server::SignalEmitter;

use super::common::read_metadata_from_cache;

lazy_static::lazy_static! {
    static ref MOVE_CANCELLATION_TOKEN: Mutex<Option<CancellationToken>> = Mutex::default();
}

fn legendary_app_to_provider_item(legendary_app: &LegendaryGameListInfo) -> ProviderItem {
    ProviderItem {
        id: legendary_app.app_name.clone(),
        name: legendary_app.app_title.to_string(),
        provider: LIBRARY_PROVIDER_ID.to_string(),
        app_type: AppType::Game,
    }
}
#[derive(Clone)]
pub struct EpicService {
    config: LegendaryConfig,
    legendary: Legendary,
    pub account: Option<String>,
}

impl EpicService {
    pub fn new() -> Self {
        let config = LegendaryConfig::new();
        Self {
            config: config.clone(),
            legendary: Legendary::new(Some(config)),
            account: None,
        }
    }

    pub async fn auth(&mut self, authorization_code: Option<String>) -> fdo::Result<()> {
        let account = self.legendary.auth(authorization_code).await;
        if account.is_err() {
            return Err(fdo::Error::Failed("Failed to authenticate".to_string()));
        }
        self.set_account().await;
        Ok(())
    }

    pub async fn initialize(&mut self) -> Result<(), String> {
        let result: Result<(), String> = require_legendary(&self.config.legendary_command)
            .await
            .map_err(|e| e.to_string());
        self.set_account().await;
        result
    }

    pub async fn set_account(&mut self) {
        let status = self.legendary.status().await.unwrap();
        self.account = Some(status.account)
    }

    pub async fn logout(&mut self) -> EmptyResult {
        match self.legendary.logout().await {
            Ok(_) => {
                self.set_account().await;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_download_size(&self, app_id: &str) -> ResultWithError<u64> {
        let game_info = self.legendary.info(app_id).await?;
        Ok(game_info.get_download_size())
    }

    pub async fn get_installed_apps(
        &self,
        emitter: SignalEmitter<'_>,
    ) -> fdo::Result<Vec<InstalledApp>> {
        match self.legendary.get_installed_apps().await {
            Ok(legendary_installed) => {
                let installed_apps = legendary_installed
                    .iter()
                    .map(|installed_app| InstalledApp {
                        app_id: installed_app.app_name.clone(),
                        installed_path: installed_app.install_path.clone(),
                        downloaded_bytes: installed_app.install_size,
                        total_download_size: installed_app.install_size,
                        disk_size: installed_app.install_size,
                        version: installed_app.version.clone(),
                        latest_version: installed_app.version.clone(),
                        update_pending: installed_app.needs_verification,
                        os: installed_app.platform.clone(),
                        disabled_dlc: vec![],
                    })
                    .collect();

                if let Ok(updated) = self
                    .legendary
                    .get_installed_apps_versions(&legendary_installed)
                    .await
                {
                    for (app_id, version) in updated {
                        if let Err(err) = emitter.app_new_version_found(app_id, version).await {
                            log::error!("Failed to notify about new version for {err}");
                        }
                    }
                }

                Ok(installed_apps)
            }
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_provider_item(&self, app_id: &str) -> fdo::Result<ProviderItem> {
        match self.legendary.info(app_id).await {
            Ok(app_info) => {
                let item = ProviderItem {
                    id: app_info.game.app_name.clone(),
                    name: app_info.game.title.to_string(),
                    provider: LIBRARY_PROVIDER_ID.to_string(),
                    app_type: AppType::Game,
                };
                Ok(item)
            }
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }
    pub async fn get_provider_items(&self) -> fdo::Result<Vec<ProviderItem>> {
        let apps = match self.legendary.get_apps().await {
            Ok(apps) => apps,
            Err(e) => {
                return Err(fdo::Error::Failed(e.to_string()));
            }
        };
        let items: Vec<ProviderItem> = apps.iter().map(legendary_app_to_provider_item).collect();
        Ok(items)
    }

    pub async fn uninstall(&self, app_id: &str) -> fdo::Result<()> {
        match self.legendary.uninstall(app_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_install_options(
        &self,
        app_id: &str,
    ) -> ResultWithError<Vec<(String, String, Vec<String>)>> {
        let metadata = read_metadata_from_cache(app_id).await?;
        let branches: Vec<String> = metadata
            .namespaces
            .into_values()
            .map(|n| n.sandbox_name)
            .collect();

        Ok(vec![(
            "branch".to_string(),
            "Branch to install from".to_string(),
            branches,
        )])
    }

    pub async fn install(
        &self,
        app_id: &str,
        base_path: String,
        options: HashMap<String, zbus::zvariant::Value<'_>>,
        emitter: SignalEmitter<'_>,
    ) -> fdo::Result<u32> {
        let verify = match options.get("verify") {
            Some(v) => v.clone().downcast::<bool>().ok(),
            None => Some(false),
        };

        let install_options = InstallOption {
            language: options
                .get("language")
                .and_then(|l| l.clone().downcast::<String>().ok()),
            os: options
                .get("os")
                .and_then(|os| os.clone().downcast::<String>().ok()),
            branch: options
                .get("branch")
                .and_then(|b| b.clone().downcast::<String>().ok()),
            verify,
        };

        match self
            .legendary
            .install(
                app_id.to_string(),
                base_path,
                install_options,
                emitter.clone(),
            )
            .await
        {
            Ok(_) => Ok(1),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn pause_install(&self) -> fdo::Result<()> {
        match self.legendary.pause_download().await {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_launch_options(&self, app_id: &str) -> fdo::Result<Vec<LaunchOption>> {
        match self.legendary.get_launch_options(app_id.to_string()).await {
            Ok(launch_options) => Ok(launch_options),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_post_install_steps(&self, app_id: String) -> fdo::Result<String> {
        let post_install_steps = self.legendary.get_prerequisites(&app_id).await;
        match post_install_steps {
            Ok(Some(post_install_steps)) => {
                let install_script: InstallScript = InstallScript {
                    path: post_install_steps.get_prerequisite_id(),
                    registry: Registry {
                        dword: None,
                        string: Some(vec![RegistryEntry {
                            group: "HKEY_CLASSES_ROOT\\com.epicgames.launcher".to_string(),
                            key: "(Default)".to_string(),
                            value: "".to_string(),
                            language: None,
                        }]),
                    },
                    run_process: [RunProcessParams {
                        name: post_install_steps.get_prerequisite_id(),
                        has_run_key: None,
                        process: post_install_steps.path,
                        command: Some(post_install_steps.args),
                        no_clean_up: None,
                        minimum_has_run_value: None,
                        requirement_os: RequirementOSConfig {
                            is_64_bit_windows: None,
                            os_type: None,
                        },
                    }]
                    .to_vec(),
                };
                Ok(
                    serde_json::to_string::<Vec<InstallScript>>(&[install_script].to_vec())
                        .unwrap(),
                )
            }
            Ok(None) => Ok("[]".to_string()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn import(&self, app_id: &str, install_folder: &str) -> fdo::Result<()> {
        match self.legendary.import(app_id, install_folder).await {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn move_item(
        &self,
        app_id: String,
        base_path: String,
        emitter: &SignalEmitter<'_>,
    ) -> ResultWithError<String> {
        let game_info = match self.legendary.get_game_info(app_id.as_str()).await {
            Ok(Some(game_info)) => game_info,
            Ok(None) => return Err("Game not installed".into()),
            Err(e) => return Err(format!("Get game info failed: {}", e).into()),
        };
        if MOVE_CANCELLATION_TOKEN.lock().is_some() {
            return Err("An app move operation is already in progress".into());
        }

        let from_path = game_info.install_path;
        let folder_name = from_path.split('/').next_back().unwrap();
        let path_buf = Path::new(&base_path).join(folder_name);
        let dest_path = path_buf.to_str().unwrap();
        log::info!("Moving from {} to {}", from_path, dest_path);

        // use util to move game folder since legendary move doesn't support moving to a different drive
        fs::remove_dir_all(&dest_path).await.unwrap_or_default();
        let mut progress = move_folder_with_progress(&from_path, dest_path).await;
        while let Some(progress) = progress.next().await {
            match progress {
                Ok(progress) => {
                    LibraryProviderSignals::move_item_progressed(
                        &emitter.clone(),
                        app_id.clone(),
                        progress.into(),
                    )
                    .await?;
                }
                Err(e) => {
                    LibraryProviderSignals::move_item_failed(
                        &emitter.clone(),
                        app_id.clone(),
                        e.to_string(),
                    )
                    .await?;
                    return Err(e);
                }
            }
        }
        self.legendary
            .move_app_to_drive(app_id.as_str(), dest_path)
            .await?;
        Ok(dest_path.to_string())
    }

    pub async fn cloud_save_download(&self, app_id: &str, force: bool) -> fdo::Result<()> {
        match self
            .legendary
            .perform_cloud_sync(app_id, CloudSyncOperation::Download, force)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn cloud_save_upload(&self, app_id: &str, force: bool) -> fdo::Result<()> {
        match self
            .legendary
            .perform_cloud_sync(app_id, CloudSyncOperation::Upload, force)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_eulas(&self, app_id: &str) -> ResultWithError<Vec<AppEulaInformation>> {
        self.legendary.get_eulas(app_id).await
    }

    pub async fn accept_eula(&self, app_id: &str) -> EmptyResult {
        self.legendary.accept_eula(app_id).await
    }

    pub async fn get_save_path_patterns(
        &self,
        app_id: String,
        platform: String,
    ) -> fdo::Result<Vec<String>> {
        let game_info = match self.legendary.info(&app_id).await {
            Ok(game_info) => game_info,
            Err(e) => return Err(fdo::Error::Failed(e.to_string())),
        };
        if platform == "mac" && game_info.game.cloud_save_folder_mac.is_some() {
            Ok([game_info.game.cloud_save_folder_mac.unwrap()].to_vec())
        } else if game_info.game.cloud_save_folder.is_some() {
            Ok([game_info.game.cloud_save_folder.unwrap()].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn pre_launch_hook(
        &self,
        app_id: &str,
        using_offline_mode: bool,
    ) -> ResultWithError<Vec<String>> {
        log::info!(
            "Pre launch hook for {} (offline mode: {})",
            app_id,
            using_offline_mode
        );

        let game_info = self.legendary.info(app_id).await?;

        let install_path = game_info.install.unwrap().install_path;
        let prefix_path = PathBuf::from(install_path)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("wine_prefixes")
            .join(app_id);
        overlay::enable_for_prefix(&self.config.clone(), &prefix_path).await?;

        let game_meta: LegendaryGameListInfo = read_metadata_from_cache(app_id).await.unwrap();
        if game_meta.get_partner_link_type() == "ubisoft" {
            let status = self.legendary.uplay_link_status().await?;
            let is_game_redeemable = status.redeemable.iter().any(|game| game.app_name == app_id);
            if is_game_redeemable {
                if let Err(err) = self.legendary.uplay_activate().await {
                    log::error!("Failed to activate game on ubisoft {err:?}");
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn get_item_metadata(&self, app_id: &str) -> String {
        let game_meta: LegendaryGameListInfo = read_metadata_from_cache(app_id).await.unwrap();
        let namespace = game_meta.metadata.namespace.to_owned();
        let images: Vec<PlaytronImage> = game_meta.get_images();
        let app_type: Option<PlaytronAppType> = game_meta.get_app_type();

        let item_meta = ItemMetadata {
            id: app_id.to_owned(),
            name: game_meta.app_title.to_owned(),
            slug: app_id.to_owned(),
            app_type: app_type.unwrap(),
            providers: vec![PlaytronProvider {
                namespace,
                provider: LIBRARY_PROVIDER_ID.to_string(),
                provider_app_id: app_id.to_owned(),
                store_id: game_meta.metadata.id.to_owned(),
                product_store_link: "https://store.epicgames.com".to_owned(),
                parent_store_id: None,
                last_imported_timestamp: None,
                known_dlc_store_ids: vec![],
            }],
            developers: vec![game_meta.metadata.developer.to_owned()],
            publishers: vec![],
            description: "".to_string(),
            summary: "".to_string(),
            tags: vec![],
            images,
        };
        serde_json::to_string(&item_meta).unwrap()
    }
}
