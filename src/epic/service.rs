use crate::constants::LIBRARY_PROVIDER_ID;
use crate::epic::signals::notify_move_item_failed;
use crate::epic::{
    common::require_legendary, config::LegendaryConfig, legendary::Legendary,
    models::LegendaryGameListInfo, signals,
};

use crate::types::app::{AppEulaInformation, AppType, InstalledApp, LaunchOption, ProviderItem};
use crate::types::cloud_sync::CloudSyncOperation;
use rand;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::LineEnding;
use rsa::{RsaPrivateKey, RsaPublicKey};

use std::collections::HashMap;
use zbus::fdo;

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
    rsa: RsaPrivateKey,
    pub account: Option<String>,
}

impl EpicService {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let config = LegendaryConfig::new();
        let rsa = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate new key");
        Self {
            config: config.clone(),
            rsa,
            legendary: Legendary::new(Some(config)),
            account: None,
        }
    }

    pub async fn auth(&mut self, authorization_code: Option<String>) -> Result<(), String> {
        let account = self.legendary.auth(authorization_code).await;
        if account.is_err() {
            return Err("Failed to authenticate".to_string());
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

    pub fn get_public_key(&self) -> String {
        let public_key = RsaPublicKey::from(&self.rsa);
        public_key.to_pkcs1_pem(LineEnding::LF).unwrap()
    }

    pub async fn set_account(&mut self) {
        let status = self.legendary.status().await.unwrap();
        self.account = Some(status.account)
    }

    pub async fn get_installed_apps(&self) -> fdo::Result<Vec<InstalledApp>> {
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
                    })
                    .collect();
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

    pub async fn get_provider_items_and_versions(
        &self,
    ) -> fdo::Result<(Vec<ProviderItem>, Vec<(String, String)>)> {
        let apps = match self.legendary.get_apps().await {
            Ok(apps) => apps,
            Err(e) => {
                return Err(fdo::Error::Failed(e.to_string()));
            }
        };
        let items: Vec<ProviderItem> = apps.iter().map(legendary_app_to_provider_item).collect();
        let app_versions = self.legendary.get_app_versions(apps);
        Ok((items, app_versions))
    }

    pub async fn uninstall(&self, app_id: &str) -> fdo::Result<()> {
        match self.legendary.uninstall(app_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn install(
        &self,
        app_id: &str,
        disk: String,
        base_path: String,
        options: HashMap<&str, &str>,
    ) -> fdo::Result<u32> {
        match self
            .legendary
            .install(app_id.to_string(), disk, base_path, options)
            .await
        {
            Ok(_) => Ok(1),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_launch_options(&self, app_id: String) -> fdo::Result<Vec<LaunchOption>> {
        match self.legendary.get_launch_options(app_id).await {
            Ok(launch_options) => Ok(launch_options),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn get_post_install_steps(&self, app_id: String) -> fdo::Result<String> {
        let post_install_steps = self.legendary.get_prerequisites(&app_id).await;
        match post_install_steps {
            Ok(post_install_steps) => Ok(serde_json::to_string(&post_install_steps).unwrap()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn import(&self, app_id: &str, install_folder: &str) -> fdo::Result<()> {
        match self.legendary.import(app_id, install_folder).await {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn move_item(&self, app_id: String, disk: String) -> fdo::Result<()> {
        match self
            .legendary
            .move_app_to_drive(app_id.as_str(), &disk, &signals::notify_move_progress)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                notify_move_item_failed(app_id, e.to_string()).await;
                Err(fdo::Error::Failed(e.to_string()))
            }
        }
    }

    pub async fn logout(&mut self) -> fdo::Result<()> {
        match self.legendary.logout().await {
            Ok(_) => {
                self.set_account().await;
                Ok(())
            }
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
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

    pub async fn get_eulas(&self, app_id: &str) -> fdo::Result<Vec<AppEulaInformation>> {
        match self.legendary.get_eulas(app_id).await {
            Ok(eulas) => Ok(eulas),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
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
}
