use zbus_macros::DBusError;

#[derive(DBusError, Debug)]
#[zbus(prefix = "one.playtron.Error")]
pub enum PluginError {
    DiskNotFound,
    DownloadInProgress,
    DownloadFailed,
    ContentNotFound,
    AppNotInstalled,
    InvalidAppId,
    NotLoggedIn,
    MissingDirectory,
    InvalidPassword,
    AuthenticationError,
    TfaTimedOut,
    RateLimitExceeded,
    AppUpdateRequired,
    DependencyUpdateRequired,
    Timeout,
    DependencyError,
    PreLaunchError,
    CloudConflict,
    CloudQuota,
    CloudFileDownload,
    CloudFileUpload,
    AppNotOwned,
    PlayingBlocked,
    NotEnoughSpace,
    Permission,
    NetworkRequired,
    #[zbus(error)]
    Generic(zbus::Error),
    MoveItemCancelled,
}
