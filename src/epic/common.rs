//! This is a common module that contains common epic apps related functions

use convert_case::{Case, Casing};
use lazy_static::lazy_static;
use regex::Regex;
use serde::de::DeserializeOwned;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::epic::models::DownloadSizeType;
use crate::plugin::errors::PluginError;
use crate::types::results::ResultWithError;
use crate::utils::{
    child::{command_with_callback, command_with_callback_json},
    system::is_program_in_path,
};

use super::models::LegendaryGameListInfo;

pub const EPIC_MAX_WRITER_QUEUE_EMPTY_ERROR_COUNT: u16 = 3;

lazy_static! {
    // Internet connection error regex
    static ref INTERNET_ERROR_REGEX: Regex =
        Regex::new(r"Failed to establish a new connection").unwrap();

    // Exchange token regex
    static ref EXCHANGE_TOKEN_ERROR_REGEX: Regex =
        Regex::new(r"errors.com.epicgames.account.oauth.authorization_code_not_found").unwrap();

    // Not logged in regex
    static ref NOT_LOGGED_IN_REGEX: Regex = Regex::new(
        r"(No saved credentials|AttributeError: 'NoneType' object has no attribute 'get')"
    )
    .unwrap();
}

pub fn get_legendary_command() -> String {
    std::env::var("LEGENDARY").unwrap_or("legendary".to_owned())
}

/// This function asserts that the legendary binary exists when running a function that needs to have legendary installed
pub async fn require_legendary(legendary_command: &str) -> Result<(), PluginError> {
    if !is_program_in_path(legendary_command).await {
        log::error!("Legendary is not installed and exported in the PATH with executable:{legendary_command}");
        return Err(PluginError::DependencyError);
    }

    Ok(())
}

/// Gets the installation directory for a epic app
pub fn get_app_install_dir(install_root: &str, name: &str) -> String {
    let mut install_path = install_root.to_string();
    install_path.push('/');
    // Remove spaces from name because Legendary also removes them
    install_path.push_str(
        name.to_case(Case::Pascal)
            .replace([' ', '\'', ':', '"', '!'], "")
            .as_str(),
    );
    install_path
}

pub fn check_invalid_legendary_response(line: &str) -> Result<(), PluginError> {
    if line.contains("errors.com.epicgames.account.auth_token.invalid_refresh_token")
        || line.contains("errors.com.epicgames.common.oauth.invalid_token")
        || NOT_LOGGED_IN_REGEX.is_match(line)
    {
        return Err(PluginError::AuthenticationError);
    }

    if line.contains("ERROR: Stored credentials are no longer valid")
        || line.contains("Login to EGS API failed")
        || EXCHANGE_TOKEN_ERROR_REGEX.is_match(line)
    {
        return Err(PluginError::NotLoggedIn);
    }

    if INTERNET_ERROR_REGEX.is_match(line) {
        return Err(PluginError::NetworkRequired);
    }

    Ok(())
}

/// Executes the legendary command and handles case where process hangs with no auth
pub async fn execute_legendary_handle_auth_raw<I, S>(
    cmd: String,
    args: I,
) -> ResultWithError<(String, String)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    command_with_callback(cmd, args, check_invalid_legendary_response).await
}

/// Executes the legendary command and handles case where process hangs with no auth
pub async fn execute_legendary_handle_auth_json<I, S, R>(cmd: String, args: I) -> ResultWithError<R>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    R: DeserializeOwned,
{
    match command_with_callback_json(cmd, args, check_invalid_legendary_response).await {
        Ok(res) => Ok(res),
        Err(err) => Err(err),
    }
}

pub fn parse_size(line: &str, download_size_type: DownloadSizeType) -> ResultWithError<f32> {
    let compare_str = if download_size_type == DownloadSizeType::Disk {
        "Install size: "
    } else {
        "Download size: "
    };

    if !line.contains(compare_str) {
        return Err("No download size found".into());
    }

    let size = line
        .split(compare_str)
        .last()
        .unwrap_or_default()
        .split(' ')
        .next()
        .unwrap_or_default();

    Ok(size.parse()?)
}

/// Read game metadata from Legendary's cache instead of querying Legendary itself
pub async fn read_metadata_from_cache(app_id: &str) -> ResultWithError<LegendaryGameListInfo> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .or_else(|_err| std::env::var("HOME").map(|var| format!("{var}/.config")))?;
    let metadata_path: PathBuf =
        format!("{config_home}/legendary/metadata/{app_id}.json").parse()?;

    if !metadata_path.exists() {
        return Err("App not found".into());
    }

    let game_info_raw = tokio::fs::read_to_string(&metadata_path).await?;
    let game_info: LegendaryGameListInfo = serde_json::from_str(&game_info_raw)?;
    Ok(game_info)
}
