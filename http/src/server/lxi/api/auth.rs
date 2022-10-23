use tide_http_auth::Scheme;

pub use tide_http_auth::{BasicAuthRequest, Storage};

#[derive(Debug, Default)]
pub struct LxiApiAuthScheme;

#[derive(Debug)]
pub struct LxiApiAuthRequest {
    pub prefix: String,
    pub token: String,
}

#[async_trait::async_trait]
impl<Permissions: Send + Sync + 'static> Scheme<Permissions> for LxiApiAuthScheme {
    type Request = LxiApiAuthRequest;

    async fn authenticate<S>(&self, state: &S, auth_param: &str) -> http_types::Result<Option<Permissions>>
    where
        S: Storage<Permissions, Self::Request> + Send + Sync + 'static,
    {
        if !auth_param.is_ascii() {
            // This is invalid. Fail the request.
            return Err(http_types::Error::from_str(
                http_types::StatusCode::Unauthorized,
                "X-API-Key must be ASCII.",
            ));
        }

        // Split the prefix and token
        let parts: Vec<_> = auth_param.split('.').collect();
        if parts.len() < 2 {
            return Ok(None);
        }

        let (prefix, token) = (parts[0], parts[1]);

        // TODO: validate that the auth_param (sans the prefix) is a valid uuid.
        let perms = state
            .get_user(LxiApiAuthRequest {
                prefix: prefix.to_owned(),
                token: token.to_owned(),
            })
            .await?;
        Ok(perms)
    }

    fn scheme_name() -> &'static str {
        ""
    }

    fn header_name() -> &'static str {
        "X-API-Key"
    }
}
