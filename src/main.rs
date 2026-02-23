use std::future::pending;
mod auth;
mod constants;
mod epic;
mod plugin;
mod types;
mod utils;

use epic::service::EpicService;

use crate::plugin::dbus::{build_connection, register_plugin};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure logging to output to stdout
    #[cfg(not(debug_assertions))]
    let level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    let level = log::LevelFilter::Debug;

    env_logger::builder()
        .filter_level(level)
        .parse_env("PLAYTRON_LOG_LEVEL")
        .target(env_logger::Target::Stdout)
        .init();

    log::info!("Starting Playtron Plugin");

    let epic_service = EpicService::new();
    epic_service.initialize().await?;
    build_connection(epic_service).await?;
    register_plugin().await;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
