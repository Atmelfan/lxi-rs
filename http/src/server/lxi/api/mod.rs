//! LXI-API end-points and
//!

pub mod prelude {
    pub use super::{
        auth::{
            LxiApiAuthRequest, LxiApiAuthStorage, LxiAuthenticationError, LxiBasicAuthRequest,
            Permission,
        },
        common_configuration::CommonConfiguration,
        device_specific_configuration::DeviceSpecificConfiguration,
    };
}

/// Endpoints for `/certificates`, `/certificates/:guid`, and `/certificates/:guid/enabled`
pub mod certificates;
/// Endpoints for `/common-configuration`
pub mod common_configuration;
/// Endpoints for `/create-certificate`
pub mod create_certificate;
/// Endpoints for `/common-configuration`
pub mod device_specific_configuration;
/// Endpoints for `/get-csr`
pub mod get_csr;

/// Authentication stuff + middleware
pub mod auth {
    use super::middleware::ProblemDetails;

    /// API permissions
    #[derive(Debug, Clone, Copy)]
    pub struct Permission {
        /// Key is allowed to add/remove users, change passwords etc.
        pub user_management: bool,
        /// Key is allowed to add/remove certificates
        pub certificate_management: bool,
    }

    impl Permission {
        ///
        pub fn admin() -> Self {
            Self {
                user_management: true,
                certificate_management: true,
            }
        }
    }

    /// Contains a LXI-API authentication token
    #[derive(Debug)]
    pub struct LxiApiAuthRequest {
        pub prefix: String,
        pub token: String,
    }

    /// Contains Http Basic authentication username and password
    #[derive(Debug)]
    pub struct LxiBasicAuthRequest {
        pub username: String,
        pub password: String,
    }

    /// Returned when failing to authenticate a [LxiApiAuthRequest] or [LxiBasicAuthRequest]
    pub enum LxiAuthenticationError {
        /// Credential are invalid, wrong, etc
        InvalidCredentials,
        /// User does not have a password set. Probably have to login and set the password through other means.
        NoPasswordSet,
        /// Some other error
        Other(String),
    }

    impl LxiAuthenticationError {
        pub fn get_error_message(&self) -> String {
            match self {
                LxiAuthenticationError::InvalidCredentials => "Invalid credentials".to_string(),
                LxiAuthenticationError::NoPasswordSet => "Password not set".to_string(),
                LxiAuthenticationError::Other(s) => s.clone(),
            }
        }
    }

    #[async_trait::async_trait]
    pub trait LxiApiAuthStorage {
        /// Get the permissions of a user.
        async fn get_user_permissions(
            &self,
            user: LxiBasicAuthRequest,
        ) -> Result<Option<Permission>, LxiAuthenticationError>;

        /// Get permissions for a specified api key. Return none if api key is not valid.
        async fn get_apikey_permissions(&self, apikey: LxiApiAuthRequest) -> Option<Permission>;
    }

    pub struct LxiApiAuthentication;

    impl LxiApiAuthentication {
        async fn authenticate_http_basic<S>(
            state: &S,
            auth_param: &str,
        ) -> tide::Result<Result<Option<Permission>, LxiAuthenticationError>>
        where
            S: LxiApiAuthStorage + Send + Sync + 'static,
        {
            let bytes = base64::decode(&auth_param.as_bytes()["Basic ".len()..]);
            if bytes.is_err() {
                // This is invalid. Fail the request.
                return Err(tide::http::Error::from_str(
                    tide::http::StatusCode::Unauthorized,
                    "Basic auth param must be valid base64.",
                ));
            }

            let as_utf8 = String::from_utf8(bytes.unwrap());
            if as_utf8.is_err() {
                // You know the drill.
                return Err(tide::http::Error::from_str(
                    tide::http::StatusCode::Unauthorized,
                    "Basic auth param base64 must contain valid utf-8.",
                ));
            }

            let as_utf8 = as_utf8.unwrap();
            let parts: Vec<_> = as_utf8.split(':').collect();

            if parts.len() < 2 {
                return Ok(Err(LxiAuthenticationError::InvalidCredentials));
            }

            let (username, password) = (parts[0], parts[1]);

            let perms = state
                .get_user_permissions(LxiBasicAuthRequest {
                    username: username.to_owned(),
                    password: password.to_owned(),
                })
                .await;
            Ok(perms)
        }

        async fn authenticate_api_key<S>(
            state: &S,
            auth_param: &str,
        ) -> tide::Result<Option<Permission>>
        where
            S: LxiApiAuthStorage + Send + Sync + 'static,
        {
            // Split the prefix and token
            let parts: Vec<_> = auth_param.split('.').collect();
            if parts.len() < 2 {
                return Ok(None);
            }

            let (prefix, token) = (parts[0], parts[1]);

            // TODO: validate that the auth_param (sans the prefix) is a valid uuid.
            let perms = state
                .get_apikey_permissions(LxiApiAuthRequest {
                    prefix: prefix.to_owned(),
                    token: token.to_owned(),
                })
                .await;
            Ok(perms)
        }
    }

    #[async_trait::async_trait]
    impl<State> tide::Middleware<State> for LxiApiAuthentication
    where
        State: LxiApiAuthStorage + Clone + Send + Sync + 'static,
    {
        async fn handle(
            &self,
            mut req: tide::Request<State>,
            next: tide::Next<'_, State>,
        ) -> tide::Result {
            // read the header
            if let Some(auth_headers) = req.header("Authorization") {
                // Authenticate Basic http
                let state = req.state();

                let auth_headers: Vec<_> = auth_headers.into_iter().collect();
                if auth_headers.len() > 1 {
                    tide::log::error!("Multiple authorization headers in request");
                    return Ok(tide::http::StatusCode::Unauthorized.into());
                }

                let header = auth_headers
                    .first()
                    .ok_or(tide::http::format_err!("Empty authentication header"))?
                    .as_str();

                if header.starts_with("Basic ") {
                    match Self::authenticate_http_basic(state, header).await? {
                        Ok(Some(p)) => {
                            // Valid credentials with api-access
                            req.set_ext(p);
                            Ok(next.run(req).await)
                        }
                        Ok(None) => {
                            // Valid credentials but no api-access
                            let mut response: tide::Response =
                                tide::http::StatusCode::Forbidden.into();
                            response.insert_ext(ProblemDetails::with_detail(
                                "No API permissions",
                                None,
                            ));
                            Ok(response)
                        }
                        Err(err) => {
                            // Invalid credentials, return 401 to allow a new attempt
                            let mut response: tide::Response =
                                tide::http::StatusCode::Unauthorized.into();
                            response.insert_header("WWW-Authenticate", "Basic realm=\"LXI-API\"");
                            response.insert_ext(ProblemDetails::with_detail(
                                err.get_error_message(),
                                None,
                            ));
                            Ok(response)
                        }
                    }
                } else {
                    // Invalid authentication method, return 401 to allow a new attempt
                    let mut response: tide::Response = tide::http::StatusCode::Unauthorized.into();
                    response.insert_header("WWW-Authenticate", "Basic realm=\"LXI-API\"");
                    Ok(response)
                }
            } else if let Some(auth_headers) = req.header("X-API-Key") {
                // Authenticate LXI-API key
                let state = req.state();

                let auth_headers: Vec<_> = auth_headers.into_iter().collect();
                if auth_headers.len() > 1 {
                    log::error!("Multiple X-API-Key headers in request");
                    return Ok(tide::Response::new(tide::http::StatusCode::Unauthorized));
                }

                let header = auth_headers
                    .first()
                    .ok_or(tide::http::format_err!("Empty X-API-Key header"))?
                    .as_str();

                if let Some(p) = Self::authenticate_api_key(state, header).await? {
                    req.set_ext(p);
                    Ok(next.run(req).await)
                } else {
                    Ok(tide::http::StatusCode::Unauthorized.into())
                }
            } else {
                // No authentication headers, ask for authentication
                let mut response: tide::Response = tide::http::StatusCode::Unauthorized.into();
                response.insert_header("WWW-Authenticate", "Basic realm=\"LXI-API\"");
                Ok(response)
            }
        }
    }
}

/// Miscellaneus middlewares
pub mod middleware {
    use crate::common::lxi::api::problem_details::LxiProblemDetails;

    /// Redirect unauthorized traffic to HTTPS (to attempt securee authentication)
    pub struct LxiProblemDetailsMiddleware;

    pub struct ProblemDetails {
        detail: Option<String>,
        instance: Option<String>,
    }

    impl ProblemDetails {
        pub fn new(detail: Option<String>, instance: Option<String>) -> Self {
            Self { detail, instance }
        }

        pub fn with_detail(detail: impl ToString, instance: Option<String>) -> Self {
            Self {
                detail: Some(detail.to_string()),
                instance,
            }
        }
    }

    #[async_trait::async_trait]
    impl<S: Clone + Send + Sync + 'static> tide::Middleware<S> for LxiProblemDetailsMiddleware {
        async fn handle(&self, req: tide::Request<S>, next: tide::Next<'_, S>) -> tide::Result {
            let mut schema = req.url().clone();

            let mut response = next.run(req).await;
            let status = response.status();
            if status.is_client_error() || status.is_server_error() || status.is_redirection() {
                schema.set_path("lxi/schemas/LXIProblemDetails/1.0");
                let details = response.ext::<ProblemDetails>();
                response.set_body(
                    LxiProblemDetails {
                        xmlns: "http://lxistandard.org/schemas/LXIProblemDetails/1.0".to_string(),
                        xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
                        xsi_schema_location: format!(
                            "http://lxistandard.org/schemas/LXIProblemDetails/1.0 {}",
                            schema.as_str()
                        ),
                        title: format!("{} - {}", status, status.canonical_reason()),
                        detail: details.and_then(|v| v.detail.clone()),
                        instance: details.and_then(|v| v.instance.clone()),
                    }
                    .to_xml()?,
                );
                response.set_content_type("application/xml");
            }
            Ok(response)
        }
    }

    /// Redirect all traffic to HTTPS if enabled
    pub struct RedirectAllHttps {
        /// Redirect *ALL* traffic to HTTPS
        pub redirect_all: bool,
        /// HTTPS port used
        pub https_port: u16,
    }

    #[async_trait::async_trait]
    impl<S: Clone + Send + Sync + 'static> tide::Middleware<S> for RedirectAllHttps {
        async fn handle(&self, req: tide::Request<S>, next: tide::Next<'_, S>) -> tide::Result {
            let mut url = req.url().clone();
            if self.redirect_all {
                let src = url.clone();
                url.set_scheme("https")
                    .map_err(|_| tide::http::format_err!("could not set scheme of url {}", url))?;
                url.set_port(Some(self.https_port))
                    .map_err(|_| tide::http::format_err!("could not set port of url {}", url))?;
                log::debug!("Redirecting client from {src} to {url}");
                Ok(tide::Redirect::new(url).into())
            } else {
                let res = next.run(req).await;
                if res.status() == tide::http::StatusCode::ImATeapot {
                    let src = url.clone();
                    url.set_scheme("https").map_err(|_| {
                        tide::http::format_err!("could not set scheme of url {}", url)
                    })?;
                    url.set_port(Some(self.https_port)).map_err(|_| {
                        tide::http::format_err!("could not set port of url {}", url)
                    })?;
                    log::debug!("Redirecting client from {src} to {url}");
                    Ok(tide::Redirect::new(url).into())
                } else {
                    Ok(res)
                }
            }
        }
    }
}
