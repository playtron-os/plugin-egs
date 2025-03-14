use zbus::fdo;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::epic::service::EpicService;

use crate::types::cloud_sync::CloudPath;
pub struct CloudSaveProvider {
    pub service: EpicService,
}

#[interface(name = "one.playtron.plugin.CloudSaveProvider")]
impl CloudSaveProvider {
    // Emitted when the sync has failed
    #[zbus(signal)]
    async fn cloud_save_sync_failed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        error: String,
        local: u64,
        remote: u64,
        quota_usage: u64,
        quota: u64,
    ) -> zbus::Result<()>;

    /// Emitted when the sync is progressing
    #[zbus(signal)]
    async fn cloud_save_sync_progressed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        progress: f32,
        sync_state: u32,
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
        app_id: String,
        platform: String,
        force: bool,
        paths: Vec<CloudPath>,
    ) -> fdo::Result<()> {
        log::info!("Downloading cloud saves for {:?} ({:?})", app_id, platform);
        log::info!("Paths {:?}", paths);
        self.service
            .cloud_save_download(app_id.as_str(), force)
            .await
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
        app_id: String,
        platform: String,
        force: bool,
        paths: Vec<CloudPath>,
    ) -> fdo::Result<()> {
        log::info!("Uploading cloud saves for {:?} ({:?})", app_id, platform);
        log::info!("Paths {:?}", paths);
        self.service.cloud_save_upload(app_id.as_str(), force).await
    }
}
