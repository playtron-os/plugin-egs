use crate::constants::LIBRARY_PROVIDER_ID;
use crate::types::app::{DownloadStage, InstalledApp, ProviderItem};
use zbus::fdo;
use zbus::object_server::SignalEmitter;
use zbus_macros::interface;

use crate::constants;
use crate::epic::overlay;
use crate::types::app::AppType;

pub struct PluginDependencies {}

#[interface(name = "one.playtron.plugin.PluginDependencies")]
impl PluginDependencies {
    /// Provider property
    #[zbus(property)]
    fn provider(&self) -> fdo::Result<String> {
        Ok(LIBRARY_PROVIDER_ID.to_string())
    }

    /// InstallAllRequiredDependencies
    fn install_all_required_dependencies(&self) -> fdo::Result<()> {
        log::info!("Install all required dependencies");
        Ok(())
    }

    fn get_dependency_item(&self, app_id: String) -> ProviderItem {
        ProviderItem {
            id: app_id,
            name: overlay::NAME.to_string(),
            provider: constants::LIBRARY_PROVIDER_ID.to_string(),
            app_type: AppType::Tool,
        }
    }

    fn get_installed_dependencies(&self) -> Vec<InstalledApp> {
        vec![]
    }

    fn get_required_dependencies(&self) -> Vec<ProviderItem> {
        vec![]
    }

    /// InstallStarted signal
    #[zbus(signal)]
    async fn install_started(
        emitter: &SignalEmitter<'_>,
        app_id: &str,
        version: &str,
        install_directory: &str,
        total_download_size: u64,
        requires_internet_connection: bool,
        os: &str,
    ) -> zbus::Result<()>;

    /// InstallCompleted signal
    #[zbus(signal)]
    async fn install_completed(emitter: &SignalEmitter<'_>, app_id: &str) -> zbus::Result<()>;

    /// InstallFailed signal
    #[zbus(signal)]
    async fn install_failed(
        emitter: &SignalEmitter<'_>,
        app_id: &str,
        error: &str,
    ) -> zbus::Result<()>;

    /// InstallProgressed signal
    #[zbus(signal)]
    async fn install_progressed(
        emitter: &SignalEmitter<'_>,
        app_id: &str,
        stage: DownloadStage,
        downloaded_bytes: u64,
        total_download_size: u64,
        progress: f64,
    ) -> zbus::Result<()>;

    /// DependencyNewVersionFound
    #[zbus(signal)]
    async fn dependency_new_version_found(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        version: String,
    ) -> zbus::Result<()>;
}
