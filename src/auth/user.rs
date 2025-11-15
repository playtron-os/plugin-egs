use zbus::object_server::SignalEmitter;
use zbus::{fdo, interface};

use crate::constants::CLIENT_PATH;
use crate::epic::service::EpicService;
use crate::plugin::dbus::CONNECTION;
use crate::types::app::ProviderStatus;
use crate::types::results::EmptyResult;

pub struct User {
    pub service: EpicService,
}

impl User {
    pub async fn emit_new_user_state_update() -> EmptyResult {
        let user_iface = {
            let connection = CONNECTION.lock().await;
            let Some(con) = connection.as_ref() else {
                return Ok(());
            };
            con.object_server()
                .interface::<_, User>(CLIENT_PATH)
                .await?
        };

        let interface = user_iface.get().await;
        interface
            .avatar_changed(user_iface.signal_emitter())
            .await?;
        interface
            .username_changed(user_iface.signal_emitter())
            .await?;
        interface
            .identifier_changed(user_iface.signal_emitter())
            .await?;
        interface
            .status_changed(user_iface.signal_emitter())
            .await?;
        Ok(())
    }
}

#[interface(name = "one.playtron.auth.User")]
impl User {
    /// An url to the avatar image. Empty if no avatar is available.
    ///
    /// busctl --user get-property one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.auth.User \
    ///   Avatar
    #[zbus(property)]
    async fn avatar(&self) -> &str {
        ""
    }

    /// Internal identifier for given user. Value provided here shouldn’t be mutable.
    /// E.g usernames that can be changed shouldn’t be used here.
    /// Empty when no user is authenticated
    ///
    /// # Example
    ///
    /// busctl --user get-property one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.auth.User \
    ///   Identifier
    #[zbus(property)]
    async fn identifier(&self) -> String {
        match self.service.user_data.lock().user_id.clone() {
            Some(id) => id,
            None => "".into(),
        }
    }

    /// The status of the provider authentication
    ///
    /// # Example
    ///
    /// busctl --user get-property one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.auth.User \
    ///   Status
    #[zbus(property)]
    async fn status(&self) -> i32 {
        let lock = self.service.user_data.lock();
        if lock.account.is_some() {
            ProviderStatus::Authorized as i32
        } else {
            ProviderStatus::Unauthorized as i32
        }
    }

    /// The displayname/username of the currently authenticated user. This will be empty
    /// if none are authenticated. Empty when no user is authenticated
    ///
    /// # Example
    ///
    /// busctl --user get-property one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.auth.User \
    ///   Username
    #[zbus(property)]
    async fn username(&self) -> String {
        match self.service.user_data.lock().account.clone() {
            Some(id) => id,
            None => "".into(),
        }
    }

    /// TODO Undocumented
    #[zbus(signal)]
    async fn auth_error(emitter: &SignalEmitter<'_>, value: String) -> zbus::Result<()>;

    /// Switch session to chosen user. Called after initialization of the
    /// plugin is done and session is available. Also used for account
    /// switching. Returns whether it was possible to re-use that session.
    ///
    /// # Example
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.auth.User \
    ///   ChangeUser "s" "PlaytronTest1"
    fn change_user(&self, user_id: String) -> fdo::Result<bool> {
        log::info!("Changing user to {}", user_id);
        Ok(false)
    }

    /// Logs specified user out of the provider.
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.EpicGames \
    ///   /one/playtron/EpicGames/LegendaryClient0 \
    ///   one.playtron.auth.User \
    ///   Logout "s" "PlaytronTest1"
    async fn logout(&mut self, user_id: String) -> fdo::Result<()> {
        log::info!("Logging out {}", user_id);
        match self.service.logout().await {
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }
}
