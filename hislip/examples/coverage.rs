use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc, time::Duration,
};

use async_std::{io, future::timeout};
use futures::lock::Mutex;
use lxi_device::{
    lock::SharedLock,
    util::{EchoDevice, SimpleDevice},
    Device,
};
use lxi_hislip::{
    server::{
        auth::{secret, PlainAuth},
        ServerBuilder,
    },
    STANDARD_PORT,
};

use clap::Parser;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;

#[cfg(feature = "tls")]
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    Certificate, NoClientAuth, PrivateKey, ServerConfig,
};
use sasl::{
    common::Identity,
    server::{Validator, ValidatorError},
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "127.0.0.1")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = STANDARD_PORT)]
    port: u16,

    /// cert file
    #[cfg(feature = "tls")]
    #[clap(short, long)]
    cert: PathBuf,

    /// key file
    #[cfg(feature = "tls")]
    #[clap(short, long)]
    key: PathBuf,

    /// Do not allow anonymous login
    #[clap(long)]
    no_anonymous: bool,
}

/// Configure the server using rusttls
/// See https://docs.rs/rustls/0.16.0/rustls/struct.ServerConfig.html for details
///
/// A TLS server needs a certificate and a fitting private key
#[cfg(feature = "tls")]
fn load_config(options: &Args) -> io::Result<ServerConfig> {
    /// Load the passed certificates file
    fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
        certs(&mut BufReader::new(File::open(path)?))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
    }
    let certs = load_certs(&options.cert)?;

    /// Load the passed keys file
    fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
        rsa_private_keys(&mut BufReader::new(File::open(path)?))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
    }
    let mut keys = load_keys(&options.key)?;

    // we don't use client authentication
    let mut config = ServerConfig::new(NoClientAuth::new());
    config
        // set this server to use one cert together with the loaded private key
        .set_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    Ok(config)
}

#[derive(Clone)]
struct DummyValidator;

impl Validator<secret::Plain> for DummyValidator {
    fn validate(&self, identity: &Identity, value: &secret::Plain) -> Result<(), ValidatorError> {
        match identity {
            Identity::None => Err(ValidatorError::AuthenticationFailed),
            Identity::Username(username) => {
                if username.eq_ignore_ascii_case("user") && value.0 == "pencil" {
                    Ok(())
                } else {
                    Err(ValidatorError::AuthenticationFailed)
                }
            }
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), io::Error> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let my_validator = DummyValidator;

    let authenticator = Arc::new(Mutex::new(PlainAuth::new(args.no_anonymous, my_validator)));

    let shared_lock0 = SharedLock::new();
    let device0: Arc<Mutex<Box<dyn Device + Send>>> =
        Arc::new(Mutex::new(Box::new(SimpleDevice::new())));

    let shared_lock1 = SharedLock::new();
    let device1: Arc<Mutex<Box<dyn Device + Send>>> = Arc::new(Mutex::new(Box::new(EchoDevice)));

    let server = ServerBuilder::default()
        .device("hislip0".to_string(), device0, shared_lock0)
        .device("hislip1".to_string(), device1, shared_lock1)
        .build_with_auth(authenticator);

    #[cfg(feature = "tls")]
    let acceptor = {
        let config = load_config(&args)?;
        TlsAcceptor::from(Arc::new(config))
    };

    println!("Running server on port {}:{}...", args.ip, args.port);
    let _res = timeout(Duration::from_millis(10000), server
    .accept(
        (&args.ip[..], args.port),
        #[cfg(feature = "tls")]
        acceptor,
    )).await;

    match _res {
        Ok(res) => res,
        Err(_) => Ok(()),
    }
}
