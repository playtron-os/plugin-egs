use crate::constants::{BUS_NAME, CLIENT_PATH};
use crate::epic::service::EpicService;
use crate::types::errors::ResultWithError;
use crate::{auth, plugin};
use zbus::{connection, zvariant::ObjectPath};

pub async fn build_connection(service: EpicService) -> ResultWithError<zbus::Connection> {
    let plugin = plugin::plugin_interface::Plugin {};
    let user = auth::user::User {
        service: service.clone(),
    };
    let cryptography = auth::cryptography::Cryptography {
        service: service.clone(),
    };
    let portal_flow = auth::portal_flow::PortalFlow {
        service: service.clone(),
    };
    let library_provider = plugin::library_provider::LibraryProvider {
        service: service.clone(),
    };
    let cloud_save_provider = plugin::cloud_save_provider::CloudSaveProvider {
        service: service.clone(),
    };

    // let plugin_dependencies = plugin::plugin_dependencies::PluginDependencies {};
    // let artwork_provider = plugin::artwork_provider::ArtworkProvider {};
    let _conn = connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(CLIENT_PATH, user)?
        .serve_at(CLIENT_PATH, cryptography)?
        .serve_at(CLIENT_PATH, portal_flow)?
        .serve_at(CLIENT_PATH, plugin)?
        .serve_at(CLIENT_PATH, library_provider)?
        .serve_at(CLIENT_PATH, cloud_save_provider)?
        //.serve_at(CLIENT_PATH, artwork_provider)?
        //.serve_at(CLIENT_PATH, plugin_dependencies)?
        .build()
        .await?;
    Ok(_conn)
}

pub async fn register_plugin(_conn: &zbus::Connection) {
    if let Err(error) = _conn
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
