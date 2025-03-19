use crate::plugin::dbus::emit_signal;
use serde::Serialize;
use zbus::zvariant::Type;

#[derive(Serialize, Type)]
struct MoveProgressBody {
    app_id: String,
    progress: f32,
}

#[derive(Serialize, Type)]
struct AppIdErrorBody {
    app_id: String,
    error: String,
}

// Installation signals

pub async fn notify_install_completed(value: String) {
    emit_signal(
        "one.playtron.plugin.LibraryProvider".to_string(),
        "install_completed".to_string(),
        value,
    )
    .await
}

pub async fn notify_install_failed(app_id: String, error: String) {
    let body = AppIdErrorBody { app_id, error };
    emit_signal(
        "one.playtron.plugin.LibraryProvider".to_string(),
        "install_completed".to_string(),
        body,
    )
    .await
}

#[derive(Serialize, Type)]
struct InstallProgressBody {
    app_id: String,
    stage: u8,
    downloaded_bytes: u64,
    total_download_size: u64,
    progress: f64,
}

pub async fn notify_install_progressed(
    app_id: String,
    stage: u8,
    downloaded_bytes: u64,
    total_download_size: u64,
    progress: f64,
) {
    let body = InstallProgressBody {
        app_id,
        stage,
        downloaded_bytes,
        total_download_size,
        progress,
    };

    emit_signal(
        "one.playtron.plugin.LibraryProvider".to_string(),
        "install_progressed".to_string(),
        body,
    )
    .await
}

// Move signals

pub async fn notify_move_progress(app_id: String, progress: u8) {
    let body = MoveProgressBody {
        app_id,
        progress: progress as f32,
    };
    emit_signal(
        "one.playtron.plugin.LibraryProvider".to_string(),
        "move_item_progressed".to_string(),
        body,
    )
    .await
}

pub async fn notify_move_item_failed(app_id: String, error: String) {
    let body = AppIdErrorBody { app_id, error };
    emit_signal(
        "one.playtron.plugin.LibraryProvider".to_string(),
        "move_item_failed".to_string(),
        body,
    )
    .await
}
