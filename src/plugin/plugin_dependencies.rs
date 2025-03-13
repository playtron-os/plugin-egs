use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::constants;
use crate::epic::overlay;
use crate::types::app::{AppType, InstalledApp, ProviderItem};
use zbus::Result;
pub struct PluginDependencies {}

#[interface(name = "one.playtron.plugin.PluginDependencies")]
impl PluginDependencies {
    #[zbus(signal)]
    async fn dependency_new_version_found(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        version: String,
    ) -> Result<()> {
        log::info!("App new version");

        Ok(())
    }

    #[zbus(signal)]
    async fn install_completed(emitter: &SignalEmitter<'_>, value: String) -> Result<()>;

    #[zbus(signal)]
    async fn install_failed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        error: String,
    ) -> Result<()>;

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
}
