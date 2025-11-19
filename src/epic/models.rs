use crate::epic::constants::{LANGUAGE_CODES, SUPPORTED_LANGUAGES};
use crate::types::app::{PlaytronAppType, PlaytronImage};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn default_empty_string() -> String {
    "".to_string()
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct LegendaryAssetInfo {
    pub app_name: String,
    pub asset_id: String,
    pub build_version: String,
    #[serde(default)]
    pub namespace: String,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone)]
pub struct LegendaryDlcInfo {
    pub app_name: String,
    pub title: String,
    pub install_size: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LegendaryCategory {
    pub path: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LegendaryCustomAttribute {
    #[serde(rename = "type")]
    pub attr_type: String,
    #[serde(default = "default_empty_string")]
    pub value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LegendaryCustomAttributes {
    #[serde(alias = "CanRunOffline")]
    pub can_run_offline: Option<LegendaryCustomAttribute>,
    #[serde(alias = "FolderName")]
    pub folder_name: Option<LegendaryCustomAttribute>,
    #[serde(alias = "CloudSaveFolder")]
    pub cloud_save_folder: Option<LegendaryCustomAttribute>,
    #[serde(alias = "partnerLinkType")]
    pub partner_link_type: Option<LegendaryCustomAttribute>,
    #[serde(alias = "ThirdPartyManagedApp")]
    pub third_party_managed_app: Option<LegendaryCustomAttribute>,
    #[serde(alias = "NeverUpdate")]
    pub never_update: Option<LegendaryCustomAttribute>,
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
    #[serde(rename = "asset_infos_v2")]
    pub asset_infos: HashMap<String, Vec<LegendaryAssetInfo>>,
    /// Metadata related to the game
    pub metadata: LegendaryMetadata,
    pub namespaces: HashMap<String, LegendaryNamespaceData>,
}

impl LegendaryGameListInfo {
    pub fn get_app_type(&self) -> Option<PlaytronAppType> {
        let mut app_type: Option<PlaytronAppType> = None;
        for category in &self.metadata.categories {
            match category.path.as_str() {
                "games" => {
                    app_type = Some(PlaytronAppType::Game);
                    break;
                }
                "addons" => {
                    app_type = Some(PlaytronAppType::Dlc);
                    break;
                }
                _ => {
                    app_type = Some(PlaytronAppType::Game);
                }
            }
        }
        let app_type = app_type.unwrap_or(PlaytronAppType::Game);
        Some(app_type)
    }

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
                    source: "EpicGames".to_string(),
                    alt: epic_image.alt,
                }
            })
            .collect()
    }

    pub fn get_install_folder(&self) -> String {
        self.metadata
            .custom_attributes
            .clone()
            .unwrap()
            .folder_name
            .unwrap()
            .value
    }

    pub fn get_partner_link_type(&self) -> Option<String> {
        self.metadata
            .custom_attributes
            .as_ref()
            .and_then(|attrs| attrs.partner_link_type.as_ref())
            .map(|link_type| link_type.value.clone())
    }

    // pub fn get_cloud_save_folder(&self) -> String {
    //     self.metadata
    //         .custom_attributes
    //         .clone()
    //         .unwrap()
    //         .cloud_save_folder
    //         .as_ref()
    //         .map_or("".to_owned(), |x| x.value.clone())
    // }
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
    pub platform_versions: HashMap<String, Vec<LegendaryAssetInfo>>,
    pub cloud_saves_supported: bool,
    pub cloud_save_folder: Option<String>,
    pub cloud_save_folder_mac: Option<String>,
    pub is_dlc: bool,
    pub launch_options: Vec<LegendaryGameLaunchOptions>,
    #[serde(default)]
    pub namespaces: Vec<LegendaryNamespaceData>,
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

    pub fn get_language_from_tag(&self, tag: &str) -> String {
        let parsed_tag = tag
            .to_lowercase()
            .replace("loc_", "")
            .replace("voice_", "")
            .replace("-", "_");

        let language = match LANGUAGE_CODES.get(parsed_tag.as_str()) {
            Some(lang) => lang.to_string(),
            None => tag.to_string(),
        };
        if SUPPORTED_LANGUAGES.contains(&language.as_str()) {
            return language;
        }
        "".to_string()
    }

    pub fn get_languages(&self) -> Vec<String> {
        self.manifest
            .install_tags
            .iter()
            .map(|tag| self.get_language_from_tag(tag))
            .filter(|x| !x.is_empty())
            .collect()
    }

    pub fn get_install_tags_for_language(&self, language: String) -> Vec<String> {
        let install_tags = self.manifest.install_tags.clone();
        if language.is_empty() || install_tags.is_empty() {
            return [].to_vec();
        }

        let mut tags: Vec<String> = [].to_vec();
        if install_tags.contains(&"".to_string()) {
            tags.push("".to_string());
        }
        let install_tag_languages: HashMap<String, String> = install_tags
            .iter()
            .map(|tag| (self.get_language_from_tag(tag), tag.clone()))
            .collect();
        if let Some(tag) = install_tag_languages.get(&language) {
            tags.push(tag.to_string())
        }
        tags
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
    pub uninstaller: Option<HashMap<String, String>>,
    pub version: String,
    pub namespace: Option<String>,
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

#[derive(Serialize, Deserialize)]
pub struct LegendaryUplayStatus {
    pub activated: Vec<LegendaryGameListInstalledInfo>,
    pub redeemable: Vec<LegendaryGameListInstalledInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct LegendaryLaunchParams {
    pub game_parameters: Vec<String>,
    pub game_executable: String,
    pub game_directory: String,
    pub egl_parameters: Vec<String>,
    pub launch_command: Vec<String>,
    pub working_directory: String,
    pub user_parameters: Vec<String>,
    pub environment: HashMap<String, String>,
    pub pre_launch_command: String,
    pub pre_launch_wait: bool,
}

#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LegendarySandboxType {
    #[default]
    Public,
    Private,
}

impl<'de> Deserialize<'de> for LegendarySandboxType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "public" => Ok(LegendarySandboxType::Public),
            "private" => Ok(LegendarySandboxType::Private),
            _ => Ok(LegendarySandboxType::Private), // fallback to Private
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct LegendaryNamespaceData {
    pub namespace: String,
    #[serde(alias = "catalogItemId")]
    pub catalog_item_id: String,
    #[serde(alias = "appName")]
    pub app_name: String,
    #[serde(alias = "productId")]
    pub product_id: String,
    #[serde(alias = "sandboxName")]
    pub sandbox_name: String,
    #[serde(alias = "sandboxType")]
    pub sandbox_type: LegendarySandboxType,
    #[serde(alias = "recordType")]
    pub record_type: String,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct LegendaryUserConfig {
    pub account_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}
