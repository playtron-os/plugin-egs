use super::config::LegendaryConfig;
use crate::epic::common::EPIC_MAX_WRITER_QUEUE_EMPTY_ERROR_COUNT;
use crate::epic::legendary::NO_INTERNET_ERROR_REGEX;
use crate::plugin::errors::PluginError;
use crate::plugin::plugin_dependencies::PluginDependenciesSignals;
use crate::types::app::DownloadStage;
use crate::types::results::{EmptyResult, ResultWithError};
use crate::utils::child::command;
use crate::utils::reaper::ReaperProcess;
use crate::utils::system::megabytes_to_bytes;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{path::PathBuf, process::Stdio};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::ChildStderr,
};
use zbus::object_server::SignalEmitter;

pub const PROVIDER_APP_ID: &str = "eos-overlay";
pub const NAME: &str = "EOS Overlay";

lazy_static::lazy_static! {
    /// Whether owned apps are being fetched
    static ref OVERLAY_REAPER: Arc<tokio::sync::Mutex<Vec<ReaperProcess>>> = Arc::new(tokio::sync::Mutex::new(Vec::new()));
}

// Remove EOS overlay
// pub async fn remove(config: LegendaryConfig) -> ResultWithError<()> {
//     let remove_arguments: Vec<String> = vec![
//         PROVIDER_APP_ID.to_owned(),
//         "remove".to_owned(),
//         "-y".to_owned(),
//     ];
//
//     let mut reaper = ReaperProcess::new(&config.legendary_command);
//     reaper.with_identifier(PROVIDER_APP_ID.into());
//     reaper.command.args(remove_arguments).stderr(Stdio::piped());
//
//     let (status, stdout, stderr) = reaper.execute_with_output(None).await?;
//
//     log::debug!(
//         "Ran eos-overlay remove command with status:{status}, stdout:{stdout}, stderr:{stderr}"
//     );
//     Ok(())
// }

/// Install EOS overlay
pub async fn install(
    config: LegendaryConfig,
    overlay_folder: String,
    emitter: SignalEmitter<'_>,
) -> ResultWithError<()> {
    let arguments: Vec<String> = vec![
        PROVIDER_APP_ID.to_owned(),
        "install".to_owned(),
        "-y".to_owned(),
        "--path".to_owned(),
        overlay_folder,
    ];
    // Execute legendary command
    let mut reaper = ReaperProcess::new(&config.legendary_command);
    reaper.with_identifier("eos-overlay".into());
    reaper.command.args(arguments).stderr(Stdio::piped());
    reaper.execute().await?;
    let stderr = reaper.stderr().await?;
    OVERLAY_REAPER.lock().await.push(reaper);
    // Get stream
    let out = BufReader::new(stderr);
    let emitter = emitter.into_owned();
    tokio::spawn(async move {
        if let Err(err) = handle_download(out, emitter.clone()).await {
            log::error!("Error during epic overlay download: err:{err:?}");

            emitter
                .install_failed(PROVIDER_APP_ID, "Unexpected server response")
                .await
                .unwrap();
            return Err(PluginError::DownloadFailed);
        }
        Ok(())
    });

    Ok(())
}

pub struct InstallConfig {
    install_folder: String,
    expected_disk_size: u64,
    download_size: u64,
}
/// Handles the download thread when download is in progress
async fn handle_download(out: BufReader<ChildStderr>, emitter: SignalEmitter<'_>) -> EmptyResult {
    log::info!("Starting new thread to read output from epic download overlay");

    let mut no_internet_error_count: Option<Instant> = None;
    let mut current_progress: f32 = 0.0;

    // If we receive this warning in the logs 2 times without any progress, we should retry download
    let mut empty_queue_warning_count = 0;

    let mut lines = out.lines();
    let mut install_config = InstallConfig {
        install_folder: "".to_string(),
        expected_disk_size: 0,
        download_size: 0,
    };
    let mut downloaded_bytes: u64 = 0;
    while let Ok(Some(line)) = lines.next_line().await {
        log::debug!("Epic Overlay Download: {}", line);

        if line.contains("No write access to") {
            return Err(PluginError::DownloadFailed.into());
        }

        if let Ok(install_dir) = parse_string("Install directory: ", None, &line) {
            install_config.install_folder.clone_from(&install_dir);
            continue;
        }

        if let Ok(install_size) = parse_float("Install size: ", None, &line) {
            install_config.expected_disk_size = megabytes_to_bytes(install_size);
            continue;
        }

        if let Ok(download_size) = parse_float("Download size: ", None, &line) {
            install_config.download_size = megabytes_to_bytes(download_size);
            continue;
        }

        if !install_config.install_folder.is_empty()
            && install_config.download_size != 0
            && install_config.expected_disk_size != 0
        {
            emitter
                .install_started(
                    PROVIDER_APP_ID,
                    "1.0",
                    &install_config.install_folder,
                    install_config.download_size,
                    true,
                    "windows",
                    "en-US",
                )
                .await
                .ok();
        }
        if let Ok(written_bytes) = parse_float("Written: ", None, &line) {
            downloaded_bytes = megabytes_to_bytes(written_bytes);
            continue;
        }

        if line.contains("WARNING: Writer queue empty!") && no_internet_error_count.is_none() {
            empty_queue_warning_count += 1;

            if empty_queue_warning_count >= EPIC_MAX_WRITER_QUEUE_EMPTY_ERROR_COUNT {
                log::error!("Legendary not progressing download of Overlay, retrying");
                break;
            }
        }

        if let Ok(progress) = parse_float("Progress: ", Some("% "), &line) {
            no_internet_error_count = None;

            if progress != current_progress {
                empty_queue_warning_count = 0;
                current_progress = progress;

                emitter
                    .install_progressed(
                        PROVIDER_APP_ID,
                        DownloadStage::Downloading,
                        downloaded_bytes,
                        install_config.download_size,
                        progress.into(),
                    )
                    .await
                    .ok();

                log::info!(
                    "Epic Overlay Download: progress:{}, stage:{:?}",
                    progress,
                    DownloadStage::Downloading
                );
            }

            continue;
        }

        if line.contains("Overlay has been downloaded")
            || line.contains("Installed version is up to date")
        {
            log::info!("Finished downloading epic overlay");
            emitter.install_completed(PROVIDER_APP_ID).await.ok();
            break;
        }

        if NO_INTERNET_ERROR_REGEX.is_match(&line) {
            match no_internet_error_count {
                None => no_internet_error_count = Some(Instant::now()),
                Some(instant) => {
                    // Give it 10 seconds to reconnect to the network
                    // Also wait for legendary to potentially restart the workers
                    if instant.elapsed() > Duration::from_secs(10) {
                        log::error!("No internet when downloading eos overlay");
                        return Err(PluginError::NetworkRequired.into());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Enables the EOS Overlay for a certain app prefix
pub async fn enable_for_prefix(config: &LegendaryConfig, prefix: &PathBuf) -> EmptyResult {
    let (_, output, status) = command(
        &config.legendary_command,
        [
            "eos-overlay",
            "enable",
            "--prefix",
            prefix.join("pfx").to_string_lossy().as_ref(),
        ],
    )
    .await?;

    if status != 0 {
        log::error!("Error enabling EOS Overlay for prefix:{prefix:?} with status:{status} and output:{output}");
    } else {
        log::info!("Enabled EOS Overlay for prefix:{prefix:?}");
    }

    Ok(())
}

fn parse_float(prefix: &str, suffix: Option<&str>, line: &str) -> ResultWithError<f32> {
    Ok(parse_string(prefix, suffix, line)?.parse()?)
}

fn parse_string(prefix: &str, suffix: Option<&str>, line: &str) -> ResultWithError<String> {
    if !line.contains(prefix) {
        return Err("Not found".into());
    }

    let value = line
        .split(prefix)
        .last()
        .unwrap_or_default()
        .split(suffix.unwrap_or(" "))
        .next()
        .unwrap_or_default();

    Ok(value.to_owned())
}
