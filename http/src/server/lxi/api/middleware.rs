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
        if status.is_client_error() {
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
        if self.redirect_all && req.url().scheme() != "https" {
            let src = url.clone();
            url.set_scheme("https")
                .map_err(|_| tide::http::format_err!("could not set scheme of url {}", url))?;
            url.set_port(Some(self.https_port))
                .map_err(|_| tide::http::format_err!("could not set port of url {}", url))?;
            log::debug!("Redirecting client from {src} to {url}");
            Ok(tide::Redirect::new(url).into())
        } else {
            Ok(next.run(req).await)
        }
    }
}

/// Redirect traffic to HTTPS (to attempt secure authentication)
pub struct HttpsGuard {
    /// HTTPS port used
    pub https_port: u16,
}

#[async_trait::async_trait]
impl<S: Clone + Send + Sync + 'static> tide::Middleware<S> for HttpsGuard {
    async fn handle(&self, req: tide::Request<S>, next: tide::Next<'_, S>) -> tide::Result {
        let mut url = req.url().clone();
        if url.scheme() != "https" {
            url.set_scheme("https")
                .map_err(|_| tide::http::format_err!("could not set scheme of url {}", url))?;
            url.set_port(Some(self.https_port))
                .map_err(|_| tide::http::format_err!("could not set port of url {}", url))?;
            log::debug!(
                "Client  attempted to access secure path '{}', redirected to {url}",
                url.path()
            );
            Ok(tide::Redirect::new(url).into())
        } else {
            Ok(next.run(req).await)
        }
    }
}
