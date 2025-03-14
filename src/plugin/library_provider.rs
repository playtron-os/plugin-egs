use std::{collections::HashMap, vec};

use zbus::fdo;
use zbus::object_server::SignalEmitter;
use zbus_macros::interface;

use crate::constants::{LIBRARY_PROVIDER_ID, LIBRARY_PROVIDER_NAME};
use crate::epic::service::EpicService;
use crate::types::app::{AppEulaInformation, InstalledApp, LaunchOption, ProviderItem};

pub struct LibraryProvider {
    pub service: EpicService,
}

#[interface(name = "one.playtron.plugin.LibraryProvider")]
impl LibraryProvider {
    /// Human readable name of the library provider
    #[zbus(property)]
    async fn name(&self) -> fdo::Result<&str> {
        Ok(LIBRARY_PROVIDER_NAME)
    }

    /// Name of the provider which this plugins represents
    /// (has to be unique between all library provider plugins)
    #[zbus(property)]
    async fn provider(&self) -> fdo::Result<&str> {
        Ok(LIBRARY_PROVIDER_ID)
    }

    /// Emitted when a installed game has a new version available to update.
    #[zbus(signal)]
    async fn app_new_version_found(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        version: String,
    ) -> zbus::Result<()>;

    /// Emitted when an install starts
    #[zbus(signal)]
    async fn install_started(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        version: String,
        install_directory: String,
        total_download_size: u64,
        requires_internet_connection: bool,
    ) -> zbus::Result<()>;

    /// Emitted when a game’s installation step is completed.
    #[zbus(signal)]
    async fn install_completed(emitter: &SignalEmitter<'_>, value: String) -> zbus::Result<()>;

    /// Emitted when a game’s installation step is failed.
    #[zbus(signal)]
    async fn install_failed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        error: String,
    ) -> zbus::Result<()>;

    /// TODO Undocumented
    #[zbus(signal)]
    async fn installed_apps_updated(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// TODO Undocumented
    #[zbus(signal)]
    async fn launch_error(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        error: String,
    ) -> zbus::Result<()>;

    /// TODO Undocumented
    #[zbus(signal)]
    async fn launch_ready(emitter: &SignalEmitter<'_>, value: String) -> zbus::Result<()>;

    /// Emitted when provider received an event about new library entries
    #[zbus(signal)]
    async fn library_updated(
        emitter: &SignalEmitter<'_>,
        new_items: &[ProviderItem],
    ) -> zbus::Result<()>;

    /// TODO Undocumented
    #[zbus(signal)]
    async fn move_item_completed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        install_folder: String,
    ) -> zbus::Result<()>;

    /// TODO Undocumented
    #[zbus(signal)]
    async fn move_item_failed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        error: String,
    ) -> zbus::Result<()>;

    /// Emitted during MoveItem, sending progress of the move.
    /// This signal is emitted to report the progress of a file move operation.
    /// The `app_id` parameter identifies the application being moved, and the `progress`
    /// parameter is a value between 0.0 and 1.0 indicating the percentage of the move
    /// that has been completed.
    #[zbus(signal)]
    async fn move_item_progressed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        progress: f32,
    ) -> zbus::Result<()>;

    async fn get_eulas(
        &self,
        app_id: String,
        country: String, // Unused
        locale: String,  // Unused
    ) -> fdo::Result<Vec<AppEulaInformation>> {
        log::info!(
            "Getting eulas for app {} ({} {})",
            &app_id,
            &country,
            &locale
        );
        match self.service.get_eulas(&app_id).await {
            Ok(eulas) => Ok(eulas),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    /// Returns a list of available install options for the given provider app id.
    /// Install options are a list of arbitrary values that the library provider knows
    /// about which can affect what version of the app is installed and how it is installed
    /// (e.g. “version”, “language”, “low-violence”, “branch”, etc.)
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   GetInstallOptions "s" "460950"
    fn get_install_options(&self, app_id: String) -> fdo::Result<Vec<String>> {
        log::info!("get_install_options for {}", app_id);
        Err(fdo::Error::Failed("Unimplemented".to_string()))
    }

    /// List all installed apps
    async fn get_installed_apps(&self) -> fdo::Result<Vec<InstalledApp>> {
        log::info!("get_installed_apps");
        self.service.get_installed_apps().await
    }

    /// Returns a list of available launch options. This includes information
    /// on how the game should be launched together with other useful metadata.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   GetLaunchOptions "s" "460950"
    fn get_launch_options(&self, app_id: String) -> fdo::Result<Vec<LaunchOption>> {
        log::info!("get_launch_options for {}", app_id);
        Err(fdo::Error::Failed("Unimplemented".to_string()))
    }

    /// TODO  Undocumented
    fn get_post_install_steps(&self, app_id: String) -> fdo::Result<String> {
        log::info!("get_post_install_steps for {}", app_id);
        Err(fdo::Error::Failed("Unimplemented".to_string()))
    }

    /// Returns a provider item for a given provider app id.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   GetProviderItem "s" "460950"
    async fn get_provider_item(&self, app_id: String) -> fdo::Result<ProviderItem> {
        self.service.get_provider_item(&app_id).await
    }

    /// Returns all provider items.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   GetProviderItems
    async fn get_provider_items(&self) -> fdo::Result<Vec<ProviderItem>> {
        self.service.get_provider_items().await
    }

    /// Start installing the app with the given provider app id to the given
    /// block device. It is the plugin’s responsibility to determine where on
    /// the mounted disk the app will be installed. Available install options
    /// can be queried by calling the GetInstallOptions method. Once the
    /// install has started, the plugin will be responsible for sending
    /// InstallProgressed signals about the progress of the install.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   Install "ssa{sv}" "460950" "/dev/sda1" 2 'os' s windows 'language' s english
    fn install(&self, app_id: &str, disk: &str, options: HashMap<&str, &str>) -> fdo::Result<u32> {
        log::info!("install {} on {}", app_id, disk);
        dbg!(options);
        Err(fdo::Error::Failed("Unimplemented".to_string()))
    }

    async fn import(&self, app_id: &str, install_folder: &str) -> fdo::Result<()> {
        self.service.import(app_id, install_folder).await
    }

    /// Moves the given app to another disk and returns the new install directory.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   MoveItem "ss" "460950" "/dev/sda3"
    async fn move_item(&self, app_id: String, disk: String) -> fdo::Result<()> {
        self.service.move_item(app_id, disk).await
    }

    /// Trigger discovery of provider items. Should emit LibraryUpdated signal if new items are discovered.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   Refresh
    fn refresh(&self) -> fdo::Result<()> {
        log::info!("refresh");
        Err(fdo::Error::Failed("Unimplemented".to_string()))
    }

    /// Uninstalls the given app.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   Uninstall "s" "460950"
    fn uninstall(&self, app_id: String) -> fdo::Result<()> {
        log::info!("uninstall {}", app_id);
        Err(fdo::Error::Failed("Unimplemented".to_string()))
    }

    /// Start updating the given game. Once the update starts,
    /// the plugin will be responsible for sending signals
    /// about the progress of the update.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   Update "s" "460950"
    fn update(&self, app_id: String) -> fdo::Result<()> {
        log::info!("update app {}", app_id);
        Ok(())
    }

    /// Obtain the list of CloudPaths applicable for given app_id and platform, if the list is empty it is assumed saves are not supported
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.LibraryProvider \
    ///   GetSavePathPatterns ss 730 windows
    fn get_save_path_patterns(&self, app_id: String, platform: String) -> fdo::Result<Vec<String>> {
        log::info!("get_save_path_patterns for {} on {}", app_id, platform);
        Ok(vec![])
    }

    fn sync_installed_apps(&self) {
        log::info!("sync installed apps");
    }

    fn pause_install(&self) {
        log::info!("pause install");
    }

    fn post_launch_hook(&self, using_offline_mode: bool) -> Vec<String> {
        log::info!("post launch hook");
        dbg!(using_offline_mode);
        vec![]
    }
}
