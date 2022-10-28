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

#[derive(Debug)]
pub struct LxiApiAuthRequest {
    pub prefix: String,
    pub token: String,
}

#[derive(Debug)]
pub struct LxiBasicAuthRequest {
    pub username: String,
    pub password: String,
}

pub enum LxiAuthenticationError {
    /// Credential are invalid
    InvalidCredentials,
    /// User does not have a password set. Probably have to login and set the password through other means.
    NoPasswordSet,
    /// Some other error
    Other,
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
            return Err(http_types::Error::from_str(
                http_types::StatusCode::Unauthorized,
                "Basic auth param must be valid base64.",
            ));
        }

        let as_utf8 = String::from_utf8(bytes.unwrap());
        if as_utf8.is_err() {
            // You know the drill.
            return Err(http_types::Error::from_str(
                http_types::StatusCode::Unauthorized,
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
                log::error!("Multiple authorization headers in request");
                return Ok(http_types::StatusCode::Unauthorized.into());
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
                        Ok(http_types::StatusCode::Forbidden.into())
                    }
                    Err(_) => {
                        // Invalid credentials, return 401 to allow a new attempt
                        let mut response: tide::Response =
                            http_types::StatusCode::Unauthorized.into();
                        response.insert_header("WWW-Authenticate", "Basic realm=\"LXI-API\"");
                        Ok(response)
                    }
                }
            } else {
                // Invalid authentication method, return 401 to allow a new attempt
                let mut response: tide::Response = http_types::StatusCode::Unauthorized.into();
                response.insert_header("WWW-Authenticate", "Basic realm=\"LXI-API\"");
                Ok(response)
            }
        } else if let Some(auth_headers) = req.header("X-API-Key") {
            // Authenticate LXI-API key
            let state = req.state();

            let auth_headers: Vec<_> = auth_headers.into_iter().collect();
            if auth_headers.len() > 1 {
                log::error!("Multiple X-API-Key headers in request");
                return Ok(tide::Response::new(http_types::StatusCode::Unauthorized));
            }

            let header = auth_headers
                .first()
                .ok_or(tide::http::format_err!("Empty X-API-Key header"))?
                .as_str();

            if let Some(p) = Self::authenticate_api_key(state, header).await? {
                req.set_ext(p);
                Ok(next.run(req).await)
            } else {
                Ok(http_types::StatusCode::Unauthorized.into())
            }
        } else {
            // No authentication headers, ask for authentication
            let mut response: tide::Response = http_types::StatusCode::Unauthorized.into();
            response.insert_header("WWW-Authenticate", "Basic realm=\"LXI-API\"");
            Ok(response)
        }
    }
}
