use http::server::lxi::api::middleware::LxiProblemDetailsMiddleware;
use http::server::lxi::identification::Identification;
use http::server::lxi::{
    self,
    api::{
        auth::{
            LxiApiAuthRequest, LxiApiAuthStorage, LxiApiAuthentication, LxiAuthenticationError,
            LxiBasicAuthRequest, Permission,
        },
        middleware::{HttpsGuard, RedirectAllHttps},
    },
};
use std::collections::HashMap;
use tide::listener::ConcurrentListener;
use tide_rustls::TlsListener;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// IP address to bind
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    /// HTTP port
    #[clap(long, default_value_t = 8080)]
    http_port: u16,

    /// HTTPS port
    #[clap(long, default_value_t = 4433)]
    https_port: u16,

    /// TLS certificate
    #[clap(short, long, default_value = "cert.pem")]
    cert: String,

    /// TLS key
    #[clap(short, long, default_value = "key.pem")]
    key: String,

    /// Redirect all HTTP traffic to HTTPS
    #[arg(long)]
    http_redirect_all: bool,

    /// Disable HTTP service
    #[arg(long)]
    http_disable: bool,
}

// We define our user struct like so:
#[derive(Clone)]
struct User {
    username: String,
    password: String,

    api_permissions: Option<Permission>,
}

// We're creating an in-memory map of usernames to users.
#[derive(Clone)]
struct ExampleState {
    users: HashMap<String, User>,
    apikeys: HashMap<String, Permission>,
}

impl ExampleState {
    pub fn new(userlist: Vec<User>, apikeys: Vec<String>) -> Self {
        let mut users = HashMap::new();
        for user in userlist {
            users.insert(user.username.to_owned(), user);
        }

        let mut api = HashMap::new();
        for apikey in apikeys {
            api.insert(apikey, Permission::admin());
        }

        ExampleState {
            users,
            apikeys: api,
        }
    }
}

impl Identification for ExampleState {
    fn lxi_version() -> String {
        "1.6".to_string()
    }

    fn manufacturer(&self) -> String {
        "Cyberdyne systems".to_string()
    }

    fn model(&self) -> String {
        "T800 Model 101".to_string()
    }

    fn serial_number(&self) -> String {
        "A9012.C".to_string()
    }

    fn interfaces(&self) -> Vec<lxi::identification::Interface> {
        vec![]
    }

    fn user_description(&self) -> String {
        "Some description".to_string()
    }

    fn host(&self) -> String {
        "localhost".to_string()
    }
}

// User permission storage
#[async_trait::async_trait]
impl LxiApiAuthStorage for ExampleState {
    async fn get_user_permissions(
        &self,
        user: LxiBasicAuthRequest,
    ) -> Result<Option<Permission>, LxiAuthenticationError> {
        match self.users.get(&user.username) {
            Some(u) => {
                if u.password == user.password {
                    Ok(u.api_permissions.clone())
                } else {
                    Err(LxiAuthenticationError::InvalidCredentials)
                }
            }
            None => Err(LxiAuthenticationError::InvalidCredentials),
        }
    }

    async fn get_apikey_permissions(&self, apikey: LxiApiAuthRequest) -> Option<Permission> {
        self.apikeys.get(&apikey.token).copied()
    }
}

impl lxi::api::common_configuration::CommonConfiguration for ExampleState {}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();
    let https_port = args.https_port;
    log::info!("Hello");

    let users = vec![
        User {
            username: "Basil".to_string(),
            password: "cool meow time".to_string(),
            api_permissions: Some(Permission::admin()),
        },
        User {
            username: "Fern".to_string(),
            password: "hunter2 am I doing this right".to_string(),
            api_permissions: None,
        },
    ];

    let mut lxi = tide::with_state(ExampleState::new(users, vec!["TOKEN".to_string()]));
    lxi.with(LxiProblemDetailsMiddleware);

    // LXI-API endpoints
    lxi.at("/identification").get(lxi::identification::get);
    lxi.at("/common-configuration")
        .get(lxi::common_configuration::get);
    lxi.at("/device-specific-configuration")
        .get(lxi::device_specific_configuration::get);

    // Requires authentication and HTTPS
    let mut api = lxi.at("/api");
    let api = api
        .with(HttpsGuard { https_port })
        .with(LxiApiAuthentication);
    api.at("common-configuration")
        .get(lxi::api::common_configuration::get)
        .put(lxi::api::common_configuration::put);
    // lxi.at("device-specific-configuration")
    //     .all(lxi::api::device_specific_configuration::get)
    //     .put(lxi::api::device_specific_configuration::put);
    // lxi.at("certificates")
    //     .get(lxi::api::certificates::get)
    //     .post(lxi::api::certificates::post);
    // lxi.at("certificates/:guid")
    //     .get(lxi::api::certificates::get_guid)
    //     .delete(lxi::api::certificates::delete_guid);
    // lxi.at("certificates/:guid/enabled")
    //     .get(lxi::api::certificates::get_enabled)
    //     .put(lxi::api::certificates::put_enabled);
    // lxi.at("get-csr").get(lxi::api::get_csr::get);
    // lxi.at("create-certificate")
    //     .get(lxi::api::create_certificate::get);

    // LXI Schemas
    let mut schemas = lxi.at("/schemas");
    schemas
        .at("LXIIdentification/1.0")
        .get(lxi::schemas::identification);
    schemas
        .at("LXICertificateList/1.0")
        .get(lxi::schemas::certificate_list);
    schemas
        .at("LXICertificateRef/1.0")
        .get(lxi::schemas::certificate_reference);
    schemas
        .at("LXICertificateRequest/1.0")
        .get(lxi::schemas::certificate_request);
    schemas
        .at("LXICommonConfiguration/1.0")
        .get(lxi::schemas::common_configuration);
    schemas
        .at("LXIDeviceSpecificConfiguration/1.0")
        .get(lxi::schemas::device_specific_configuration);
    schemas.at("LXILiterals/1.0").get(lxi::schemas::literals);
    schemas
        .at("LXIPendingDetails/1.0")
        .get(lxi::schemas::pending_details);
    schemas
        .at("LXIProblemDetails/1.0")
        .get(lxi::schemas::problem_details);

    // Root server
    let mut app = tide::Server::new();
    app.with(RedirectAllHttps {
        https_port,
        redirect_all: args.http_redirect_all,
    });
    app.at("/").get(welcome);

    app.at("/lxi").nest(lxi);

    app.listen(
        ConcurrentListener::new()
            .with_listener((args.ip.clone(), args.http_port))
            .with_listener(
                TlsListener::build()
                    .addrs((args.ip.clone(), args.https_port))
                    .cert(args.cert)
                    .key(args.key)
                    .finish()?,
            ),
    )
    .await?;
    Ok(())
}

async fn welcome<State>(_req: tide::Request<State>) -> tide::Result<tide::Response> {
    Ok("Welcome".into())
}

async fn secure<State>(req: tide::Request<State>) -> tide::Result<tide::Response> {
    if let Some(perms) = req.ext::<Permission>() {
        Ok(format!("API access ok, permissions = {perms:?}").into())
    } else {
        let mut response: tide::Response = "API access denied".to_string().into();
        response.set_status(tide::http::StatusCode::Unauthorized);
        Ok(tide::http::StatusCode::Unauthorized.into())
    }
}
