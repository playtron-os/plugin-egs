use crate::constants::ALLOWED_ROUTES;
use crate::epic::service::EpicService;
use zbus::{fdo, interface};
pub struct PortalFlow {
    pub service: EpicService,
}

/// Interface for defining how to log in to a provider using a portal flow
/// where a webview is opened to get an authorization token.
#[interface(name = "one.playtron.auth.PortalFlow")]
impl PortalFlow {
    /// Log in to the provider with the given username and password
    ///
    /// extra could be any stringified data. A token, cookie or a
    /// more advanced JSON object. It is up to the plugin on how
    /// the payload will look like
    async fn login(&mut self, url: String, _extra: String) -> fdo::Result<()> {
        log::info!("Logging in with :{}", &url);
        let parsed = url::Url::parse(&url).map_err(|err| fdo::Error::Failed(err.to_string()))?;
        let code = parsed
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, v)| v.to_string());
        self.service.auth(code).await?;
        Ok(())
    }

    /// The url used to initiate the auth flow.
    #[zbus(property)]
    async fn url(&self) -> &str {
        "https://www.epicgames.com/id/login?responseType=code"
    }

    /// RegExp determining if the flow is finished.
    /// This supports JavaScript syntax of expressions
    #[zbus(property)]
    async fn final_url_pattern(&self) -> &str {
        "^https://localhost/"
    }

    /// JavaScript code that will run after final url is reached. Returned
    /// data will be sent over as an extra parameter for login. If empty,
    /// no extra data will be obtained
    #[zbus(property)]
    async fn on_finish_script(&self) -> &str {
        ""
    }

    /// A list of routes that are allowed for the flow e.g https://auth.gog.com.
    /// Note if any route that’s not on the list is reached the flow will be
    /// cancelled resulting in auth error.
    #[zbus(property)]
    async fn allowed_routes(&self) -> Vec<&str> {
        ALLOWED_ROUTES.to_vec()
    }

    #[zbus(property)]
    fn user_agent(&self) -> &str {
        "EpicGamesLauncher"
    }

    #[zbus(property)]
    fn show_library_sync_view(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
