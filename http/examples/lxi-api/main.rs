use futures::try_join;
use http::server::lxi::{
    self,
    api::{middleware::RedirectAllHttps, prelude::Permission},
};

use clap::Parser;

use crate::state::User;

mod state;

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
    #[clap(short, long, default_value = ".certificates/cert.pem")]
    cert: String,

    /// TLS key
    #[clap(short, long, default_value = ".certificates/key.pem")]
    key: String,

    /// Redirect all HTTP traffic to HTTPS
    #[arg(long)]
    http_redirect_all: bool,

    /// Disable HTTP service
    #[arg(long)]
    http_disable: bool,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();
    let https_port = args.https_port;

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

    let mystate = state::MyState::new(users, vec!["TOKEN".to_string()]);

    // Http server
    let insecure = {
        let mut app = tide::Server::new();
        app.with(RedirectAllHttps {
            https_port,
            redirect_all: args.http_redirect_all,
        });
        app.at("/").get(welcome);
        app.at("/lxi")
            .nest(lxi::LxiService::new_http_api(mystate.clone()).0);
        app.listen((args.ip.clone(), args.http_port))
    };

    // Https server
    let secure = {
        let mut app = tide::Server::new();
        app.at("/").get(welcome);
        app.at("/lxi")
            .nest(lxi::LxiService::new_https_api(mystate.clone()).0);
        app.listen(
            tide_rustls::TlsListener::build()
                .addrs((args.ip.clone(), args.https_port))
                .cert(args.cert)
                .key(args.key)
                .finish()?,
        )
    };

    // Run until one crashes
    try_join!(insecure, secure)?;
    Ok(())
}

async fn welcome<State>(_req: tide::Request<State>) -> tide::Result<tide::Response> {
    Ok("Welcome".into())
}
