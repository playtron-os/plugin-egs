use num_derive::FromPrimitive;
use thiserror::Error;

#[derive(Copy, Clone, Error, Debug, PartialEq, FromPrimitive)]
pub enum PluginError {
    #[error("one.playtron.Error.DiskNotFound")]
    DiskNotFound,
    #[error("one.playtron.Error.DownloadInProgress")]
    DownloadInProgress,
    #[error("one.playtron.Error.DownloadFailed")]
    DownloadFailed,
    #[error("one.playtron.Error.ContentNotFound")]
    ContentNotFound,
    #[error("one.playtron.Error.AppNotInstalled")]
    AppNotInstalled,
    #[error("one.playtron.Error.InvalidAppId")]
    InvalidAppId,
    #[error("one.playtrong.Error.NotLoggedIn")]
    NotLoggedIn,
    #[error("one.playtrong.Error.MissingDirectory")]
    MissingDirectory,
    #[error("one.playtrong.Error.InvalidPassword")]
    InvalidPassword,
    #[error("one.playtrong.Error.AuthenticationError")]
    AuthenticationError,
    #[error("one.playtrong.Error.TfaTimedOut")]
    TfaTimedOut,
    #[error("one.playtrong.Error.RateLimitExceeded")]
    RateLimitExceeded,
    #[error("one.playtrong.Error.AppUpdateRequired")]
    AppUpdateRequired,
    #[error("one.playtrong.Error.DependencyUpdateRequired")]
    DependencyUpdateRequired,
    #[error("one.playtrong.Error.Timeout")]
    Timeout,
    #[error("one.playtrong.Error.DependencyError")]
    DependencyError,
    #[error("one.playtrong.Error.PreLaunchError")]
    PreLaunchError,
    #[error("one.playtrong.Error.CloudConflict")]
    CloudConflict,
    #[error("one.playtrong.Error.CloudQuota")]
    CloudQuota,
    #[error("one.playtrong.Error.CloudFileDownload")]
    CloudFileDownload,
    #[error("one.playtrong.Error.CloudFileUpload")]
    CloudFileUpload,
    #[error("one.playtrong.Error.AppNotOwned")]
    AppNotOwned,
    #[error("one.playtrong.Error.PlayingBlocked")]
    PlayingBlocked,
    #[error("one.playtrong.Error.NotEnoughSpace")]
    NotEnoughSpace,
    #[error("one.playtrong.Error.Permission")]
    Permission,
    #[error("one.playtrong.Error.NetworkRequired")]
    NetworkRequired,
}
