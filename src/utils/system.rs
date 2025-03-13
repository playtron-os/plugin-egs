use crate::types::errors::U8Result;
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
) -> Pin<Box<impl Stream<Item = U8Result>>> {
    info!("Moving folder from: {} to: {}", from, to);

    let from = String::from(from);
    let to = String::from(to);

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

        let output = command.wait_with_output().await?;

        if !output.status.success() {
            let stdout = std::str::from_utf8(&output.stdout)?;
            let err = std::str::from_utf8(&output.stderr)?;
            log::error!("Error moving app {}, status:{}, stdout:{}", err, output.status, stdout);
            // TODO: call on_move_failed
        }

        fs::remove_dir_all(&from).await?;
    };

    Box::pin(stream)
}
