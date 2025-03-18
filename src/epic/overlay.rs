#![allow(dead_code)]
use std::{path::PathBuf, process::Stdio};
use tokio::{io::BufReader, process::ChildStderr};

use super::config::LegendaryConfig;
use crate::types::results::{EmptyResult, ResultWithError};
use crate::utils::child::command;
use crate::utils::reaper::ReaperProcess;

pub const PROVIDER_APP_ID: &str = "eos-overlay";
pub const NAME: &str = "EOS Overlay";

/// Remove EOS overlay
pub async fn remove(config: LegendaryConfig) -> ResultWithError<()> {
    let remove_arguments: Vec<String> = vec![
        PROVIDER_APP_ID.to_owned(),
        "remove".to_owned(),
        "-y".to_owned(),
    ];

    let mut reaper = ReaperProcess::new(&config.legendary_command);
    reaper.with_identifier(PROVIDER_APP_ID.into());
    reaper.command.args(remove_arguments).stderr(Stdio::piped());

    let (status, stdout, stderr) = reaper.execute_with_output(None).await?;

    log::debug!(
        "Ran eos-overlay remove command with status:{status}, stdout:{stdout}, stderr:{stderr}"
    );
    Ok(())
}

/// Install EOS overlay
pub async fn install(
    config: LegendaryConfig,
    overlay_folder: String,
) -> ResultWithError<BufReader<ChildStderr>> {
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
    // Get stream
    let out = BufReader::new(stderr);
    Ok(out)
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
