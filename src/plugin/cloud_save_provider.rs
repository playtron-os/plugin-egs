use zbus::fdo;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::epic::service::EpicService;

use crate::types::cloud_sync::CloudPath;
use crate::types::cloud_sync::CloudSyncFailed;
use crate::types::cloud_sync::CloudSyncProgress;
pub struct CloudSaveProvider {
    pub service: EpicService,
}

#[interface(name = "one.playtron.plugin.CloudSaveProvider")]
impl CloudSaveProvider {
    // Emitted when the sync has failed
    #[zbus(signal)]
    async fn cloud_save_sync_failed(
        emitter: &SignalEmitter<'_>,
        failure: CloudSyncFailed,
    ) -> zbus::Result<()>;

    /// Emitted when the sync is progressing
    #[zbus(signal)]
    async fn cloud_save_sync_progressed(
        emitter: &SignalEmitter<'_>,
        progress: CloudSyncProgress,
    ) -> zbus::Result<()>;

    /// Start downloading the save data for the given provider app id using the given path patterns.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.CloudSaveProvider \
    ///   CloudSaveDownload "ssba(sssbas)" "460950" 1 "/path/to/saves"
    async fn cloud_save_download(
        &mut self,
        app_id: &str,
        platform: &str,
        force: bool,
        paths: Vec<CloudPath>,
    ) -> fdo::Result<()> {
        log::info!("Downloading cloud saves for {:?} ({:?})", app_id, platform);
        log::info!("Paths {:?}", paths);
        self.service.cloud_save_download(app_id, force).await
    }

    /// Start uploading the save data for the given provider app
    /// id using the given path patterns.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.plugin.CloudSaveProvider \
    ///   CloudSaveUpload "ssba(sssbas)" "460950" 1 "/path/to/saves"
    async fn cloud_save_upload(
        &mut self,
        app_id: &str,
        platform: &str,
        force: bool,
        paths: Vec<CloudPath>,
    ) -> fdo::Result<()> {
        log::info!("Uploading cloud saves for {:?} ({:?})", app_id, platform);
        log::info!("Paths {:?}", paths);
        self.service.cloud_save_upload(app_id, force).await
    }
}
