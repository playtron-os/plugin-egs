use crate::{plugin::errors::PluginError, types::results::U8Result};
use async_stream::try_stream;
use futures::Stream;
use log::{info, trace};
use regex::Regex;
use std::{env, path::PathBuf, pin::Pin, process::Stdio};
use tokio::{
    fs::{self},
    io::AsyncReadExt,
    process::Command,
};
use tokio_util::sync::CancellationToken;

/// Verifies if a program is in the PATH
pub async fn is_program_in_path(program: &str) -> bool {
    if program.starts_with('/') {
        return PathBuf::from(program).exists();
    }

    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, program);
            trace!("Checking if {} exists", p_str);
            if fs::metadata(p_str).await.is_ok() {
                return true;
            }
        }
    }
    false
}

/// Move app from one directory to another
pub async fn move_folder_with_progress(
    from: &str,
    to: &str,
    cancellation: CancellationToken,
) -> Pin<Box<impl Stream<Item = U8Result>>> {
    info!("Moving folder from: {} to: {}", from, to);

    let mut from = String::from(from);
    let to = String::from(to);
    if !from.ends_with('/') {
        from.push('/');
    }

    let stream = try_stream! {
        // ensure the new folder exists
        fs::create_dir_all(&to).await?;

        // move the files to the new location
        let mut command = Command::new("rsync")
            .arg("-a")
            .arg("--info=progress2")
            .arg(&from)
            .arg(&to)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = command.stdout.as_mut().unwrap();

        let progress_regex = Regex::new(r"\s(\d+)%\s").unwrap();

        let mut buffer = [0; 1];
        let mut line = Vec::new();

        let mut last_progress = 0;
        while stdout.read(&mut buffer).await? > 0 {
            if buffer[0] == b'\r' || buffer[0] == b'\n' {
                let progress = String::from_utf8_lossy(&line);

                if let Some(Ok(progress)) = progress_regex
                    .captures(&progress)
                    .and_then(|cap| cap.get(1))
                    .map(|progress| progress.as_str().parse::<u8>())
                {
                    if progress != last_progress {
                        last_progress = progress;
                        yield progress;
                    }
                }

                line.clear(); // Clear the buffer after printing
                continue;
            }

            // Collect the bytes of the line
            line.push(buffer[0]);
        }

       let res = tokio::select! {
            Ok(status) = command.wait() => {
                if !status.success() {
                    log::error!("Error moving app status:{}", status);
                    Err(tokio::io::Error::other("bad status code"))
                }
                else {
                    fs::remove_dir_all(&from).await
                }
            },
            _ = cancellation.cancelled() => {
                Err(tokio::io::Error::new(std::io::ErrorKind::Interrupted, "interrupted"))
            }
        };

        if let Err(err) = res {
            if matches!(err.kind(), std::io::ErrorKind::Interrupted) {
                command.kill().await?;
                Err(PluginError::MoveItemCancelled)?
            }
        }
    };

    Box::pin(stream)
}

pub fn bytes_to_megabytes(bytes: u64) -> f32 {
    bytes as f32 / (1024.0 * 1024.0)
}

pub fn megabytes_to_bytes(megabytes: f32) -> u64 {
    (megabytes * 1024.0 * 1024.0) as u64
}
