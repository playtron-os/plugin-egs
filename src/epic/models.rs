// #![allow(dead_code)]
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    constants::LIBRARY_PROVIDER_ID,
    types::app::{AppType, InstalledApp, PlaytronImage},
};

fn default_empty_string() -> String {
    "".to_string()
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct LegendaryAssetInfo {
    pub app_name: String,
    pub asset_id: String,
    pub build_version: String,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct LegendaryDlcInfo {
    pub app_name: String,
    pub title: String,
    pub install_size: u64,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct LegendaryCategory {
    pub path: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryCustomAttribute {
    #[serde(rename = "type")]
    pub attr_type: String,
    #[serde(default = "default_empty_string")]
    pub value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryCustomAttributes {
    #[serde(alias = "CanRunOffline")]
    pub can_run_offline: Option<LegendaryCustomAttribute>,
    #[serde(alias = "FolderName")]
    pub folder_name: Option<LegendaryCustomAttribute>,
    #[serde(alias = "CloudSaveFolder")]
    pub cloud_save_folder: Option<LegendaryCustomAttribute>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LegendaryKeyImage {
    pub width: u32,
    pub height: u32,
    pub url: String,
    #[serde(alias = "type")]
    pub image_type: String,
    #[serde(default)]
    pub alt: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct LegendaryMetadata {
    /// Categories that this app belong to
    pub categories: Vec<LegendaryCategory>,
    #[serde(alias = "customAttributes")]
    pub custom_attributes: Option<LegendaryCustomAttributes>,
    pub id: String,
    pub namespace: String,
    #[serde(alias = "creationDate")]
    pub creation_date: String,
    pub developer: String,
    #[serde(default, alias = "keyImages")]
    pub key_images: Vec<LegendaryKeyImage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LegendaryGameListInfo {
    /// Id of the app
    pub app_name: String,
    /// Title/name of the app
    pub app_title: String,
    /// Map of asset infos where key is the platform such as "Windows"
    pub asset_infos: HashMap<String, LegendaryAssetInfo>,
    /// Metadata related to the game
    pub metadata: LegendaryMetadata,
}

impl LegendaryGameListInfo {
    #[allow(dead_code)]
    pub fn get_app_type(&self) -> Option<AppType> {
        let mut app_type: Option<AppType> = None;
        for category in &self.metadata.categories {
            match category.path.as_str() {
                "games" => {
                    app_type = Some(AppType::Game);
                    break;
                }
                "addons" => {
                    app_type = Some(AppType::Dlc);
                    break;
                }
                _ => (),
            }
        }
        let app_type = app_type.unwrap_or(AppType::Game);
        Some(app_type)
    }

    #[allow(dead_code)]
    pub fn get_images(&self) -> Vec<PlaytronImage> {
        self.metadata
            .key_images
            .clone()
            .into_iter()
            .map(|epic_image| {
                let image_type = match epic_image.image_type.as_str() {
                    "DieselGameBox" => "OfferImageWide",
                    "DieselGameBoxTall" => "OfferImageTall",
                    _ => &epic_image.image_type,
                }
                .to_owned();
                PlaytronImage {
                    url: epic_image.url,
                    image_type,
                    source: LIBRARY_PROVIDER_ID.to_string(),
                    alt: epic_image.alt,
                }
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct LegendaryGameListInstalledInfo {
    pub app_name: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryGameLaunchOptions {
    pub name: String,
    pub parameters: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryGame {
    pub app_name: String,
    pub title: String,
    pub version: String,
    pub platform_versions: HashMap<String, String>,
    pub cloud_saves_supported: bool,
    pub cloud_save_folder: Option<String>,
    pub cloud_save_folder_mac: Option<String>,
    pub is_dlc: bool,
    pub launch_options: Vec<LegendaryGameLaunchOptions>,
    // TODO: Add proper type, need to find a game with this property
    // pub owned_dlc: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryGameManifestPrerequisites {
    pub ids: Vec<String>,
    pub name: String,
    pub path: String,
    pub args: String,
}

impl LegendaryGameManifestPrerequisites {
    #[allow(dead_code)]
    pub fn get_prerequisite_id(&self) -> String {
        self.ids
            .first()
            .cloned()
            .unwrap_or(format!("Epic Prerequisite {}", self.name))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryTag {
    pub tag: String,
    pub size: u64,
    pub count: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryGameManifest {
    pub size: u64,
    #[serde(alias = "type")]
    pub manifest_type: String,
    pub version: u32,
    pub feature_level: u32,
    pub app_name: String,
    pub launch_exe: String,
    pub launch_command: String,
    pub build_version: String,
    pub build_id: String,
    pub install_tags: Vec<String>,
    pub num_files: u32,
    pub num_chunks: u32,
    pub disk_size: u64,
    pub download_size: u64,
    pub prerequisites: Option<LegendaryGameManifestPrerequisites>,
    #[serde(default)]
    pub tag_disk_size: Vec<LegendaryTag>,
    #[serde(default)]
    pub tag_download_size: Vec<LegendaryTag>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryGameInstall {
    pub platform: String,
    pub version: String,
    pub disk_size: u64,
    pub install_path: String,
    pub save_path: Option<String>,
    pub synced_egl_guid: String,
    pub install_tags: Vec<String>,
    pub requires_ovt: bool,
    pub installed_dlc: Vec<LegendaryDlcInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryGameInfo {
    pub game: LegendaryGame,
    pub manifest: LegendaryGameManifest,
    pub install: Option<LegendaryGameInstall>,
}

impl LegendaryGameInfo {
    pub fn get_disk_size(&self) -> u64 {
        self.manifest
            .tag_disk_size
            .iter()
            .find(|x| x.tag.is_empty())
            .map(|x| x.size)
            .unwrap_or(self.manifest.disk_size)
    }

    pub fn get_download_size(&self) -> u64 {
        self.manifest
            .tag_download_size
            .iter()
            .find(|x| x.tag.is_empty())
            .map(|x| x.size)
            .unwrap_or(self.manifest.download_size)
    }

    #[allow(dead_code)]
    pub fn to_installed_app(&self) -> InstalledApp {
        InstalledApp {
            app_id: self.game.app_name.clone(),
            installed_path: "".to_string(),
            downloaded_bytes: 0,
            total_download_size: self.get_download_size(),
            disk_size: self.get_disk_size(),
            version: self.game.version.clone(),
            latest_version: self.game.version.clone(),
            update_pending: false,
            os: "windows".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryEula {
    pub key: String,
    pub version: i32,
    pub revision: i32,
    pub locale: String,
    pub title: String,
    pub body: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LegendaryInstalledGameResponse {
    pub app_name: String,
    pub base_urls: Vec<String>,
    pub can_run_offline: bool,
    pub egl_guid: String,
    pub executable: String,
    pub install_path: String,
    pub install_size: u64,
    pub install_tags: Vec<String>,
    pub is_dlc: bool,
    pub launch_parameters: String,
    pub manifest_path: Option<String>,
    pub needs_verification: bool,
    pub platform: String,
    pub prereq_info: Option<String>,
    pub requires_ot: bool,
    pub save_path: Option<String>,
    pub title: String,
    pub uninstaller: Option<String>,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
/// Response to epic status request
pub struct EpicStatusResponse {
    /// Account will be the display name of the epic account, and if it is empty, it means the user is logged out
    pub account: String,
    pub games_available: i32,
    pub games_installed: i32,
    pub egl_sync_enabled: bool,
    pub config_directory: String,
}

#[derive(PartialEq)]
pub enum DownloadSizeType {
    Disk,
    Download,
}
