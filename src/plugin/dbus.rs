use crate::constants::{BUS_NAME, CLIENT_PATH};
use crate::epic::service::EpicService;
use crate::types::results::EmptyResult;
use crate::{auth, plugin};
use tokio::sync::Mutex;
use zbus::{connection, zvariant::ObjectPath, Connection};

lazy_static::lazy_static! {
    pub static ref CONNECTION: Mutex<Option<Connection>> =
        Mutex::new(None);
}

pub async fn build_connection(service: EpicService) -> EmptyResult {
    let plugin = plugin::plugin_interface::Plugin {};
    let user = auth::user::User {
        service: service.clone(),
    };
    let api_token_flow = auth::api_token::ApiToken {
        service: service.clone(),
    };
    let library_provider = plugin::library_provider::LibraryProvider {
        service: service.clone(),
    };
    let plugin_dependencies = plugin::plugin_dependencies::PluginDependencies {};
    let cloud_save_provider = plugin::cloud_save_provider::CloudSaveProvider {
        service: service.clone(),
    };

    *CONNECTION.lock().await = Some(
        connection::Builder::session()?
            .name(BUS_NAME)?
            .serve_at(CLIENT_PATH, user)?
            .serve_at(CLIENT_PATH, api_token_flow)?
            .serve_at(CLIENT_PATH, plugin)?
            .serve_at(CLIENT_PATH, library_provider)?
            .serve_at(CLIENT_PATH, cloud_save_provider)?
            .serve_at(CLIENT_PATH, plugin_dependencies)?
            .build()
            .await?,
    );
    Ok(())
}

pub async fn register_plugin() {
    if let Some(conn) = CONNECTION.lock().await.as_ref() {
        if let Err(error) = conn
            .call_method(
                Some("one.playtron.Playserve"),
                "/one/playtron/plugins/Manager",
                Some("one.playtron.plugin.Manager"),
                "RegisterPlugin",
                &(BUS_NAME, ObjectPath::try_from(CLIENT_PATH).unwrap()),
            )
            .await
        {
            log::error!("The plugin failed to register: {}", error);
        } else {
            log::info!("The plugin registered successfully");
        }
    }
}
