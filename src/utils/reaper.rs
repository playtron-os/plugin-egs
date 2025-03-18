#![allow(dead_code)]
use std::{
    collections::HashMap,
    ffi::OsStr,
    pin::Pin,
    process::{ExitStatus, Stdio},
    sync::Arc,
    time::Duration,
};

use tokio::sync::Mutex;

use futures::Future;
use serde::de::DeserializeOwned;
use tokio::{
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    plugin::errors::PluginError,
    types::results::{EmptyResult, ResultWithError},
    utils::child::{
        command, deserialize_json, get_children_pids, sig_continue, sig_kill, sig_stop,
        sig_terminate, stdio_from_raw_fd, stdio_from_raw_fd_with_ownership,
    },
};

pub enum ReaperCommand {
    Terminate,
}

lazy_static::lazy_static! {
    /// Used to hold stop/cont functions so we don't run them concurrently to avoid issues, because they are run in separate threads
    static ref IS_REAPER_STOP_CONT_EXECUTING: Arc<tokio::sync::Mutex<bool>> = Arc::new(tokio::sync::Mutex::new(false));
    static ref REAPER_TX_MAP: tokio::sync::Mutex<HashMap<u32, mpsc::Sender<ReaperCommand>>> = tokio::sync::Mutex::new(HashMap::new());
    static ref REAPER_EXIT_CODE_MAP: tokio::sync::Mutex<HashMap<u32, mpsc::Sender<ReaperCommand>>> = tokio::sync::Mutex::new(HashMap::new());
    static ref REAPER_TERMINATING_MAP: tokio::sync::Mutex<HashMap<u32, bool>> = tokio::sync::Mutex::new(HashMap::new());
}

const IDENTIFIER_KEY: &str = "Identifier";

/// Interval to use when checking if processes have been killed, in milliseconds
const KILL_CHECK_INTERVAL: u64 = 50;
/// After trying to terminate processes, we will check in a loop with [KILL_CHECK_INTERVAL] for this amount of iterations
/// and if processes are still alive, we forcefully kill them
/// ex: the value 100 represents 5 seconds
const DEFAULT_MAX_KILL_FAIL_COUNT: u64 = 100;
const DEFAULT_EXIT_CODE: i32 = 0;
/// Custom status code used to denote an error during reaper termination
const INVALID_STATUS_CODE: i32 = -2;

pub type OutputCallback = fn(&str) -> Result<(), PluginError>;

#[derive(Debug)]
pub struct ReaperProcess {
    pub command: std::process::Command,
    shutdown_callback: Option<fn() -> JoinHandle<()>>,
    identifier: Option<String>,
    raw_fd: Option<i32>,
    max_kill_fail_count: u64,
    exit_code: Option<i32>,
    is_terminated: Arc<parking_lot::Mutex<bool>>,
    pid: u32,
    child_stdout: Option<ChildStdout>,
    child_stderr: Option<ChildStderr>,
    terminate_receiver: Option<mpsc::Receiver<i32>>,
}

impl Clone for ReaperProcess {
    fn clone(&self) -> Self {
        let mut new_command = std::process::Command::new(self.command.get_program());
        if let Some(current_dir) = self.command.get_current_dir() {
            new_command.current_dir(current_dir);
        }
        for (key, value) in self.command.get_envs() {
            if let Some(value) = value {
                new_command.env(key, value);
            }
        }
        new_command.args(self.command.get_args());

        Self {
            command: new_command,
            shutdown_callback: self.shutdown_callback,
            identifier: self.identifier.clone(),
            raw_fd: self.raw_fd,
            max_kill_fail_count: self.max_kill_fail_count,
            exit_code: None,
            is_terminated: self.is_terminated.clone(),
            pid: 0,
            child_stdout: None,
            child_stderr: None,
            terminate_receiver: None,
        }
    }
}

impl ReaperProcess {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        ReaperProcess {
            command: std::process::Command::new(program),
            shutdown_callback: None,
            identifier: None,
            raw_fd: None,
            max_kill_fail_count: DEFAULT_MAX_KILL_FAIL_COUNT,
            exit_code: None,
            is_terminated: Arc::new(parking_lot::Mutex::new(false)),
            pid: 0,
            child_stdout: None,
            child_stderr: None,
            terminate_receiver: None,
        }
    }

    pub fn new_with_command(command: std::process::Command) -> Self {
        Self {
            command,
            shutdown_callback: None,
            identifier: None,
            raw_fd: None,
            max_kill_fail_count: DEFAULT_MAX_KILL_FAIL_COUNT,
            exit_code: None,
            is_terminated: Arc::new(parking_lot::Mutex::new(false)),
            pid: 0,
            child_stdout: None,
            child_stderr: None,
            terminate_receiver: None,
        }
    }

    pub async fn send_command_pid(pid: u32, cmd: ReaperCommand) {
        if let Some(tx) = REAPER_TX_MAP.lock().await.get(&pid).cloned() {
            tx.send(cmd).await.unwrap_or_default();
        }
    }

    pub async fn send_command(&self, cmd: ReaperCommand) {
        Self::send_command_pid(self.pid, cmd).await;
    }

    pub async fn is_running(&self) -> bool {
        Self::is_running_pid(&self.pid).await
    }

    pub async fn is_running_pid(pid: &u32) -> bool {
        REAPER_TX_MAP.lock().await.get(pid).is_some()
    }

    pub fn with_identifier(&mut self, identifier: String) -> &mut Self {
        self.identifier = Some(identifier);
        self
    }

    #[allow(dead_code)]
    pub fn with_raw_fd(&mut self, raw_fd: i32) -> &mut Self {
        self.raw_fd = Some(raw_fd);
        self
    }

    async fn verify_started(&self) -> EmptyResult {
        if !self.is_running().await {
            return Err("Reaper is not running".into());
        }

        Ok(())
    }

    pub fn get_reaper_pid(&self) -> u32 {
        self.pid
    }

    async fn spawn(&mut self) -> ResultWithError<Child> {
        if self.is_running().await {
            return Err("Reaper is already running".into());
        }

        // Start new reaper command
        let mut reaper_command = Command::new("reaper");

        if let Some(identifier) = self.identifier.as_ref() {
            reaper_command.arg(format!("{IDENTIFIER_KEY}={identifier}"));
        }

        if let Some(raw_fd) = self.raw_fd {
            reaper_command
                .stdin(stdio_from_raw_fd_with_ownership(raw_fd)?)
                .stdout(stdio_from_raw_fd(raw_fd)?)
                .stderr(stdio_from_raw_fd(raw_fd)?);
        } else {
            reaper_command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::piped());
        }

        // Add child command
        if let Some(current_dir) = self.command.get_current_dir() {
            reaper_command.current_dir(current_dir);
        }
        reaper_command.arg("--");
        for (key, value) in self.command.get_envs() {
            if let Some(value) = value {
                reaper_command.env(key, value);
            }
        }
        reaper_command.arg(self.command.get_program());
        reaper_command.args(self.command.get_args());

        // Execute
        let child = reaper_command.spawn()?;
        let pid = child.id().unwrap_or_default();

        if pid == 0 {
            log::error!("Reaper PID is 0");
            return Err("Error starting reaper".into());
        }

        log::debug!(
            "Executing reaper (pid: {:?}) with command: {reaper_command:?}",
            pid
        );

        Ok(child)
    }

    pub async fn execute(&mut self) -> EmptyResult {
        let mut child = self.spawn().await?;
        let pid = child.id().unwrap_or_default();
        self.pid = pid;
        self.child_stderr = child.stderr.take();
        self.child_stdout = child.stdout.take();

        let (tx, mut rc) = mpsc::channel::<ReaperCommand>(128);
        REAPER_TX_MAP.lock().await.insert(self.pid, tx);

        let shutdown_callback = self.shutdown_callback.take();
        let max_kill_fail_count = self.max_kill_fail_count;
        let identifier = self.identifier.clone();

        let (terminate_tx, terminate_receiver) = mpsc::channel::<i32>(128);
        self.terminate_receiver = Some(terminate_receiver);

        tokio::spawn(async move {
            if let Some(cmd) = rc.recv().await {
                match cmd {
                    ReaperCommand::Terminate => {
                        *REAPER_TERMINATING_MAP
                            .lock()
                            .await
                            .entry(pid)
                            .or_insert(false) = true;

                        match Self::terminate_pid(
                            pid,
                            identifier.clone(),
                            &mut child,
                            max_kill_fail_count,
                            shutdown_callback,
                        )
                        .await
                        {
                            Ok(status_code) => {
                                terminate_tx.send(status_code).await.unwrap_or_default();
                            }
                            Err(err) => {
                                log::error!("Error when terminating reaper with pid:{pid} and identifier:{identifier:?}, err:{err:?}");
                                terminate_tx
                                    .send(INVALID_STATUS_CODE)
                                    .await
                                    .unwrap_or_default();
                            }
                        }

                        *REAPER_TERMINATING_MAP
                            .lock()
                            .await
                            .entry(pid)
                            .or_insert(false) = false;
                    }
                }
            }

            child.kill().await.unwrap_or_default();
            child.wait().await.unwrap_or_default();
            REAPER_TX_MAP.lock().await.remove(&pid);

            log::debug!("Reaper process with pid:{pid} and identifier:{identifier:?} terminated");
        });

        log::debug!(
            "Reaper process executed with pid:{} and identifier:{:?}",
            self.pid,
            self.identifier
        );

        Ok(())
    }

    pub async fn execute_with_output(
        &mut self,
        callback: Option<OutputCallback>,
    ) -> ResultWithError<(i32, String, String)> {
        let child = self.spawn().await?;

        let output = child.wait_with_output().await?;
        let status = output.status.code().unwrap_or(1);
        let stdout = std::str::from_utf8(&output.stdout)?;
        let stderr = std::str::from_utf8(&output.stderr)?;

        if let Some(callback) = callback {
            callback(stdout)?;
            callback(stderr)?;
        }

        Ok((status, stdout.to_owned(), stderr.to_owned()))
    }

    pub async fn execute_with_output_json<R>(
        &mut self,
        callback: Option<OutputCallback>,
    ) -> ResultWithError<R>
    where
        R: DeserializeOwned,
    {
        let (_, output, _) = self.execute_with_output(callback).await?;
        deserialize_json::<R>(output)
    }

    pub async fn stdout(&mut self) -> ResultWithError<ChildStdout> {
        self.verify_started().await?;
        self.child_stdout
            .take()
            .ok_or("Error unwrapping reaper child stdout".into())
    }

    pub async fn stderr(&mut self) -> ResultWithError<ChildStderr> {
        self.verify_started().await?;
        self.child_stderr
            .take()
            .ok_or("Error unwrapping reaper child stderr".into())
    }

    pub async fn stop(&mut self) -> EmptyResult {
        log::info!(
            "Stopping reaper for pid:{}, identifier:{:?}...",
            self.pid,
            self.identifier
        );

        execute_on_child_pids(self.pid, sig_stop).await;

        Ok(())
    }

    pub async fn cont(&mut self) -> EmptyResult {
        log::info!(
            "Continuing reaper for pid:{}, identifier:{:?}...",
            self.pid,
            self.identifier
        );

        execute_on_child_pids(self.pid, sig_continue).await;

        Ok(())
    }

    pub async fn terminate(&mut self) -> ResultWithError<i32> {
        // In case of reaper already being terminated, wait for it to terminate
        let reaper_pid = self.pid;
        tokio::time::timeout(Duration::from_secs(5), async move {
            while REAPER_TERMINATING_MAP
                .lock()
                .await
                .get(&reaper_pid)
                .cloned()
                .unwrap_or_default()
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_default();

        if let Some(code) = self.exit_code {
            return Ok(code);
        }

        if self.verify_started().await.is_err() {
            return Ok(0);
        }

        *self.is_terminated.lock() = true;

        self.send_command(ReaperCommand::Terminate).await;

        if let Some(mut rc) = self.terminate_receiver.take() {
            if let Some(status_code) = rc.recv().await {
                if status_code != INVALID_STATUS_CODE {
                    self.exit_code = Some(status_code);
                    return Ok(status_code);
                }
            }
        }

        Err(format!("Error terminating reaper with pid:{}", self.pid).into())
    }

    async fn terminate_pid(
        reaper_pid: u32,
        identifier: Option<String>,
        child: &mut Child,
        max_kill_fail_count: u64,
        shutdown_callback: Option<fn() -> JoinHandle<()>>,
    ) -> ResultWithError<i32> {
        log::info!(
            "Terminating reaper for pid:{}, identifier:{:?}...",
            reaper_pid,
            identifier
        );

        let mut children_pids = get_children_pids(reaper_pid, true).await?;
        children_pids.reverse();

        if !children_pids.is_empty() {
            if let Some(callback) = shutdown_callback.as_ref() {
                log::info!("Running shutdown callback for reaper process");
                if let Err(err) = callback().await {
                    log::error!("Error on shutdown callback, err:{err:?}");
                }
            }
        }

        // Termimnate all children pids in a leaf-first approach
        for child_pid in children_pids.iter() {
            sig_continue(child_pid.to_string()).await;
            sig_terminate(child_pid.to_string()).await;
        }

        // Kill winedevice.exe which hangs sometimes
        let (all_processes, _, _) = command("ps", ["-e", "-o", "pid,comm"]).await?;
        for child_pid in children_pids {
            if all_processes.contains(&format!("{} winedevice.exe", child_pid)) {
                log::info!("Killing winedevice.exe process forcefully with pid:{child_pid}");
                sig_kill(child_pid.to_string()).await;
            }
        }

        child.kill().await.unwrap_or_default();

        let mut kill_check_count = 0;

        loop {
            tokio::time::sleep(Duration::from_millis(KILL_CHECK_INTERVAL)).await;

            let children_pids = get_children_pids(reaper_pid, false).await?;

            if !children_pids.is_empty() {
                kill_check_count += 1;

                if kill_check_count >= max_kill_fail_count {
                    let mut children_pids = get_children_pids(reaper_pid, true).await?;
                    children_pids.reverse();

                    for child_pid in children_pids {
                        sig_kill(child_pid.to_string()).await;
                    }

                    break;
                }
            } else {
                break;
            }
        }

        // Wait for reaper to exit
        let exit_status: ExitStatus = child.wait().await?;
        let exit_code = exit_status.code();

        log::info!("Successfully terminated reaper with pid:{reaper_pid}");

        Ok(exit_code.unwrap_or(DEFAULT_EXIT_CODE))
    }

    fn _drop(&mut self) {
        log::debug!(
            "Dropping reaper with pid:{} and identifier:{:?}",
            self.pid,
            self.identifier
        );
        let pid = self.pid;
        let identifier = self.identifier.clone();

        tokio::spawn(async move {
            if let Some(tx) = REAPER_TX_MAP.lock().await.get(&pid).cloned() {
                tx.send(ReaperCommand::Terminate).await.unwrap_or_default();
            }

            log::debug!(
                "Dropped reaper with pid:{} and identifier:{:?}",
                pid,
                identifier
            );
        });
    }
}

impl Drop for ReaperProcess {
    fn drop(&mut self) {
        self._drop();
    }
}

pub type OnExecuteOnChildPidsAsyncFn =
    fn(String) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

async fn execute_on_child_pids(reaper_pid: u32, callback: OnExecuteOnChildPidsAsyncFn) {
    tokio::spawn(async move {
        wait_for_mutex(IS_REAPER_STOP_CONT_EXECUTING.clone(), Some(2)).await;
        *IS_REAPER_STOP_CONT_EXECUTING.lock().await = true;

        let mut children_pids = get_children_pids(reaper_pid, true)
            .await
            .unwrap_or_default();
        children_pids.reverse();

        // Stop all children pids in a leaf-first approach
        for child_pid in children_pids.iter() {
            callback(child_pid.to_string()).await;
        }

        *IS_REAPER_STOP_CONT_EXECUTING.lock().await = false;
    });
}

/// Waits until a mutex is unlocked and returns whether it was locked, times out at 5 seconds by default
pub async fn wait_for_mutex(mutex: Arc<Mutex<bool>>, timeout_secs: Option<u16>) -> bool {
    let mut was_locked = false;
    let timeout_secs = timeout_secs.unwrap_or(5);

    tokio::time::timeout(Duration::from_secs(timeout_secs as u64), async {
        while *mutex.lock().await {
            was_locked = true;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_default();

    was_locked
}
