use std::net::ToSocketAddrs;

use tide_rustls::TlsListener;

pub mod lxi;

pub const DEFAULT_HTTP_PORT: u16 = 80;
pub const DEFAULT_HTTPS_PORT: u16 = 443;

pub struct ServiceConfig {
    name: String,
    enabled: bool,
    basic_auth: (),
    digest_auth: (),
}

pub struct HttpConfig {
    enable: bool,
    redirect_all: bool,
    port: u16,
}

pub struct HttpServer<S> {
    config: HttpConfig,
    server: tide::Server<S>,
}

impl<S> HttpServer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(config: HttpConfig, state: S) -> Self {
        let server = tide::Server::with_state(state);
        Self { config, server }
    }

    pub async fn accept<T>(self, addr: &str) -> Result<(), std::io::Error>
    where
        (T, u16): ToSocketAddrs,
    {
        self.server.listen((addr, self.config.port)).await
    }
}

impl<S> std::ops::Deref for HttpServer<S> {
    type Target = tide::Server<S>;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}

impl<S> std::ops::DerefMut for HttpServer<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.server
    }
}

// #[cfg(feature = "tls")]
// pub struct HttpsConfig {
//     enable: bool,
//     port: u16,
// }

// #[cfg(feature = "tls")]
// pub struct HttpsServer<S> {
//     config: HttpsConfig,
//     server: tide::Server<S>,
// }

// #[cfg(feature = "tls")]
// impl<S> HttpsServer<S>
// where
//     S: Clone + Send + Sync + 'static,
// {
//     pub fn new(config: HttpsConfig, state: S) -> Self {
//         let server = tide::Server::with_state(state);
//         Self { config, server }
//     }

//     pub async fn accept<T>(self, addr: &str) -> Result<(), std::io::Error>
//     where
//         (T, u16): ToSocketAddrs,
//     {
//         let tls_listener = tide_rustls::TlsListener::build().config(config)
//             .addrs((addr, self.config.port))
//             .cert(args.cert)
//             .key(args.key)
//             .finish()?;
//         self.server.listen(TlsListener).await
//     }
// }

// struct LxiTlsListener<S> {
//     serverconfig: (),
//     server: Option<tide::Server<S>>
// }

// #[async_trait::async_trait]
// impl<S> tide::listener::Listener<S> for LxiTlsListener<S>
// where
//     S: Send + Sync + 'static,
// {
//     async fn bind(&mut self, app: tide::Server<S>) -> std::io::Result<()> {
//         self.as_mut().bind(app).await
//     }

//     async fn accept(&mut self) -> std::io::Result<()> {
//         self.as_mut().accept().await
//     }

//     fn info(&self) -> Vec<tide::listener::ListenInfo> {
//         self.as_ref().info()
//     }
// }

// #[cfg(feature = "tls")]
// impl<S> std::ops::Deref for HttpsServer<S> {
//     type Target = tide::Server<S>;

//     fn deref(&self) -> &Self::Target {
//         &self.server
//     }
// }

// #[cfg(feature = "tls")]
// impl<S> std::ops::DerefMut for HttpsServer<S> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.server
//     }
// }
