use crate::epic::service::EpicService;
use zbus::fdo;
use zbus_macros::interface;

pub struct PasswordFlow {
    pub service: EpicService,
}

#[interface(name = "one.playtron.auth.PasswordFlow")]
impl PasswordFlow {
    /// Log in to the provider with the given username and password
    ///
    /// # Example
    ///
    /// busctl --user call one.playtron.SteamBus \
    ///   /one/playtron/SteamBus/SteamClient0 \
    ///   one.playtron.auth.PasswordFlow \
    ///   Login "ss" "PlaytronTest1" "mysecretpassword"
    async fn login(&mut self, name: String, password: String) -> fdo::Result<()> {
        match self.service.login(name, password).await {
            Ok(()) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }
}
