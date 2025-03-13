use crate::constants::LIBRARY_PROVIDER_ID;
use crate::epic::{common::require_legendary, config::LegendaryConfig, legendary::Legendary};
use crate::types::app::{AppEulaInformation, AppType, InstalledApp, ProviderItem};
use crate::types::cloud_sync::CloudSyncOperation;
use crate::types::errors::{EmptyResult, ResultWithError};
use rand;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::LineEnding;
use rsa::{RsaPrivateKey, RsaPublicKey};
use zbus::fdo;

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

    pub async fn get_installed_apps(&self) -> ResultWithError<Vec<InstalledApp>> {
        log::info!("Getting installed apps");
        let installed_apps = self.legendary.get_installed_apps().await?;
        Ok(installed_apps
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
            .collect())
    }

    pub async fn get_provider_item(&self, app_id: &str) -> ResultWithError<ProviderItem> {
        log::info!("get_provider_item for {}", app_id);
        let res = self.legendary.info(app_id).await;
        match res {
            Ok(app_info) => {
                let item = ProviderItem {
                    id: app_info.game.app_name.clone(),
                    name: app_info.game.title.to_string(),
                    provider: LIBRARY_PROVIDER_ID.to_string(),
                    app_type: AppType::Game,
                };
                dbg!(app_info);
                Ok(item)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn import(&self, app_id: &str, install_folder: &str) {
        let res = self.legendary.import(app_id, install_folder).await;
        match res {
            Ok(_) => (),
            Err(e) => log::error!(
                "Failed to import {:?} to {:?}: {}",
                app_id,
                install_folder,
                e
            ),
        }
    }

    pub async fn move_item(&self, app_id: String, disk: String) -> Result<(), fdo::Error> {
        log::info!("Moving {:?} to {:?}", app_id, disk);

        let result = self
            .legendary
            .move_app_to_drive(app_id.as_str(), &disk)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn logout(&mut self) -> Result<(), fdo::Error> {
        log::info!("Logging out");
        let res = self.legendary.logout().await;
        match res {
            Ok(_) => {
                self.set_account().await;
                Ok(())
            }
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    pub async fn cloud_save_download(&self, app_id: &str, force: bool) -> EmptyResult {
        self.legendary
            .perform_cloud_sync(app_id, CloudSyncOperation::Download, force)
            .await
    }

    pub async fn get_eulas(&self, app_id: &str) -> fdo::Result<Vec<AppEulaInformation>> {
        match self.legendary.get_eulas(app_id).await {
            Ok(eulas) => Ok(eulas),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }
}
