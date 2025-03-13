use crate::constants::{ALLOWED_ROUTES, CLIENT_ID};
use crate::epic::service::EpicService;
use zbus::interface;
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
    async fn login(&mut self, url: String, extra: String) {
        log::info!("Logging in with :{}", &url);
        self.service.auth(Some(extra)).await.unwrap()
    }

    /// The url used to initiate the auth flow.
    #[zbus(property)]
    async fn url(&self) -> String {
        let login_url = format!(
            "https://www.epicgames.com/id/api/redirect?clientId={CLIENT_ID}&responseType=code"
        );
        login_url
    }

    ///  JavaScript boolean expression determining if the flow is finished
    #[zbus(property)]
    async fn is_finished_script(&self) -> &str {
        "document.body?.firstChild?.tagName === 'PRE' || \
        (location.href.startsWith('https://www.epicgames.com/site') && \
        location.href.endsWith('/home'))"
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

    /// A beginning of the id corresponding to the input field corresponding to
    /// two factor codes. It is optional can can be left empty
    #[zbus(property)]
    async fn two_factor_input_id_prefix(&self) -> &str {
        ""
    }
}
