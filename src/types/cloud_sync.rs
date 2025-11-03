use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

#[derive(Debug, Serialize, Deserialize, PartialEq, Type)]
#[serde(rename_all = "lowercase")]
pub enum CloudSyncOperation {
    Download,
    Upload,
}

// Plugin stuff

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct CloudSyncProgress {
    pub app_id: String,
    pub progress: f64,
    pub sync_state: CloudSyncOperation,
}

#[derive(Serialize, Deserialize, Debug, Type)]
pub struct CloudSyncFailed {
    pub app_id: String,
    pub error: String,
    pub local: u64,
    pub remote: u64,
    pub usage: u64,
    pub quota: u64,
}

#[derive(Serialize, Deserialize, Debug, Type, Clone)]
pub struct CloudPath {
    pub alias: String,
    pub path: String,
    pub pattern: String,
    pub recursive: bool,
    pub platforms: Vec<String>,
}
