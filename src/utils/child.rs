use super::strings::regex_capture;
use crate::{plugin::errors::PluginError, types::errors::ResultWithError};

use futures::Future;
use lazy_static::lazy_static;
// use nix::unistd::dup;
use crate::{
    // models::errors::{ApiError, ResultWithError, StringResult},
    // tools::common::get_tools_dir,
    utils::strings::StringUtils,
};
use nix::unistd::dup;
use parking_lot::Mutex;
use regex::Regex;
use serde::de::DeserializeOwned;
use std::{
    // fs::Permissions,,
    ffi::OsStr,
    os::fd::{FromRawFd, RawFd},
    pin::Pin,
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    // fs,
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};

lazy_static! {
    /// Json match regex
    static ref JSON_REGEX: Regex = Regex::new(r"(?s)\{.*\}").unwrap();

    /// Json array match regex
    static ref JSON_ARRAY_REGEX: Regex = Regex::new(r"(?s)\[.*\]").unwrap();
}

/// Get stdio from raw fd
pub fn stdio_from_raw_fd_with_ownership(raw_fd: RawFd) -> ResultWithError<Stdio> {
    match dup(raw_fd) {
        Ok(duplicated_fd) => Ok(unsafe { Stdio::from_raw_fd(duplicated_fd) }),
        Err(e) => {
            log::error!("Error when converting raw file descriptor to stdio, err:{e:?}");
            Err("Bad file descriptor".into())
        }
    }
}

/// Get cloned stdio from raw fd
pub fn stdio_from_raw_fd(raw_fd: RawFd) -> ResultWithError<Stdio> {
    match dup(raw_fd) {
        Ok(duplicated_fd) => Ok(unsafe { Stdio::from_raw_fd(duplicated_fd) }),
        Err(e) => {
            log::error!("Error when converting raw file descriptor to stdio, err:{e:?}");
            Err("Bad file descriptor".into())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Sig {
    Kill = 9,
    Term = 15,
    Cont = 18,
    Stop = 19,
}

/// Gets the child process ids of a process id
pub fn get_children_pids(
    pid: u32,
    recursive: bool,
) -> Pin<Box<dyn Future<Output = ResultWithError<Vec<u32>>> + Sync + Send>> {
    Box::pin(async move {
        if pid == 0 {
            return Ok(vec![]);
        }

        let mut children_pids = vec![];
        let mut command = Command::new("pgrep");
        let command = command.arg("-P").arg(pid.to_string());

        let output = command.output().await;
        if let Err(err) = output {
            log::error!("Error getting children pids, err:{}", err);
            return Ok(vec![]);
        }

        let output = output?;
        let stdout = std::str::from_utf8(&output.stdout)?;
        let new_children_pids = stdout
            .split('\n')
            .map(|x| x.parse::<u32>().unwrap_or(0))
            .filter(|x| *x != 0)
            .collect::<Vec<u32>>();

        children_pids.extend(new_children_pids);

        if recursive {
            for child_pid in children_pids.clone() {
                let mut grandchildren_pids = get_children_pids(child_pid, true).await?;
                children_pids.append(&mut grandchildren_pids);
            }
        }

        Ok(children_pids)
    })
}

/// Signals a child process by name using the OS `pkill` command
async fn terminate_inner(pid: &str, sig: Sig) {
    let sig = sig as usize;
    let sig = format!("-{sig}");

    match command("kill", [&sig, pid]).await {
        Err(err) => {
            log::error!(
                "Error signaling {} process with signal {}, error:{}",
                pid,
                sig,
                err.to_string()
            );
        }
        Ok(_) => {
            log::debug!("Signalled process pid:{} with signal:{}", pid, sig);
        }
    }
}

/// Terminates a child process by sending a kill signal to it
pub fn sig_terminate(pid: String) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    Box::pin(async move {
        terminate_inner(&pid, Sig::Term).await;
    })
}

/// Terminates a child process forcefully
pub fn sig_kill(pid: String) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    Box::pin(async move {
        terminate_inner(&pid, Sig::Kill).await;
    })
}

pub fn sig_continue(pid: String) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    Box::pin(async move {
        terminate_inner(&pid, Sig::Cont).await;
    })
}

pub fn sig_stop(pid: String) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    Box::pin(async move {
        terminate_inner(&pid, Sig::Stop).await;
    })
}

/// Deserializes a json string into a struct
pub fn deserialize_json<R>(string: String) -> ResultWithError<R>
where
    R: DeserializeOwned,
{
    let regex = match string.trim_start().starts_with('[') {
        true => JSON_ARRAY_REGEX.to_owned(),
        false => JSON_REGEX.to_owned(),
    };
    let capture = regex_capture(regex, string);
    if capture.is_empty() {
        return Err("Error executing command".into());
    }

    let response: R = serde_json::from_str(capture.as_str())?;

    Ok(response)
}

/// Generic function to run a new command with some arguments
/// Returns stdout and stderr
pub async fn command<I, S, Z>(cmd: Z, args: I) -> ResultWithError<(String, String, i32)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    Z: AsRef<OsStr>,
{
    let output = Command::new(cmd)
        .args(args)
        .env("LANG", "C")
        .output()
        .await?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;

    Ok((
        stdout.to_owned(),
        stderr.to_owned(),
        output.status.code().unwrap_or(1),
    ))
}

/// Generic function to run a new command with some arguments and returns a parsed JSON response
/// Accepts a callback parameter that is called on every line received from the process, and if an error is returned, the process will terminate
pub async fn command_with_callback_json<I, S, R>(
    cmd: String,
    args: I,
    callback: fn(&str) -> Result<(), PluginError>,
) -> ResultWithError<R>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    R: DeserializeOwned,
{
    let (output, _) = command_with_callback(cmd, args, callback).await?;
    deserialize_json::<R>(output)
}

/// Generic function to run a new command with some arguments and returns the output string
/// Accepts a callback parameter that is called on every line received from the process, and if an error is returned, the process will terminate
pub async fn command_with_callback<I, S>(
    cmd: String,
    args: I,
    callback: fn(&str) -> Result<(), PluginError>,
) -> ResultWithError<(String, String)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = Command::new(&cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;

    let cmd_clone = cmd.clone();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());
    let mut output = String::new();
    let err_output = Arc::new(Mutex::new("".to_owned()));
    let err_output_clone = err_output.clone();
    let pid = child.id();
    let pid_clone = pid;
    let (tx, mut rc) = mpsc::channel::<Option<PluginError>>(128);

    tokio::spawn(async move {
        let mut stderr_lines = stderr.lines();
        while let Ok(Some(line)) = stderr_lines.next_line().await {
            let line = line.as_str();

            log::debug!("{} stderr: {}", cmd_clone, line);
            *err_output_clone.lock() += line;

            if let Err(err) = callback(line) {
                if let Some(pid) = pid_clone {
                    sig_kill(pid.to_string()).await;
                }
                tx.send(Some(err)).await.unwrap_or_default();
                return;
            }
        }

        tx.send(None).await.unwrap_or_default();
    });

    let mut stdout_lines = stdout.lines();
    while let Ok(Some(line)) = stdout_lines.next_line().await {
        // let line = line.as_str();

        log::debug!("{} stdout: {}", cmd, line.substring(0, 10000));

        output += "\n";
        output += line.as_str();

        if let Err(err) = callback(line.as_str()) {
            if let Some(pid) = pid {
                sig_kill(pid.to_string()).await;
            }
            child.wait().await?;
            return Err(err.into());
        }
    }

    child.wait().await?;

    if let Ok(Some(Some(err))) = tokio::time::timeout(Duration::from_secs(5), rc.recv()).await {
        return Err(err.into());
    }

    let err_output = err_output.lock();
    let err = err_output.trim().to_owned();

    Ok((output.trim().to_owned(), err))
}

/// Run sync OS command to sync cached file system changes
pub async fn command_sync() {
    match command("sync", Vec::<String>::new()).await {
        Ok((_, _, status)) => log::debug!("Sync command ran with status:{status}"),
        Err(err) => log::error!("Sync command failed with err:{err:?}"),
    };
}
