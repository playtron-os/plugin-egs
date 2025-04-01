use zbus::fdo;
use zbus_macros::interface;

use crate::epic::service::EpicService;

pub struct ApiToken {
    pub service: EpicService,
}

/// Interface for defining how to log in to a provider using an API Token
#[interface(name = "one.playtron.auth.ApiTokenFlow")]
impl ApiToken {
    /// Logs the user in using an API token
    async fn login(&mut self, api_token: String) -> fdo::Result<()> {
        self.service.auth(Some(api_token)).await
    }
}
