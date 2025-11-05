use crate::constants;
use crate::epic::config::LegendaryConfig;
use crate::epic::overlay;
use crate::types::app::{AppType, DownloadStage, InstalledApp, ProviderItem, ReleaseState};
use zbus::fdo;
use zbus::object_server::SignalEmitter;
use zbus_macros::interface;

pub struct PluginDependencies {}

#[interface(name = "one.playtron.plugin.PluginDependencies")]
impl PluginDependencies {
    /// Provider property
    #[zbus(property)]
    fn provider(&self) -> fdo::Result<String> {
        Ok(constants::LIBRARY_PROVIDER_ID.to_string())
    }

    /// InstallAllRequiredDependencies
    async fn install_all_required_dependencies(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        log::info!("Install all required dependencies");
        let config = LegendaryConfig::new();
        let data_dir = dirs::data_dir().unwrap();
        let overlay_folder = data_dir.join("playtron/tools/eos-overlay");
        match overlay::install(
            config,
            overlay_folder.to_str().unwrap().to_string(),
            emitter,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    fn get_dependency_item(&self) -> ProviderItem {
        ProviderItem {
            id: overlay::PROVIDER_APP_ID.to_string(),
            name: overlay::NAME.to_string(),
            provider: constants::LIBRARY_PROVIDER_ID.to_string(),
            app_type: AppType::Tool,
            release_state: ReleaseState::Released,
            release_date: 0,
        }
    }

    fn get_installed_dependencies(&self) -> Vec<InstalledApp> {
        vec![]
    }

    fn get_required_dependencies(&self) -> Vec<ProviderItem> {
        vec![self.get_dependency_item()]
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
