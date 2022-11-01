use std::{
    fs::File,
    io::{self, BufReader},
    sync::Arc,
    time::Duration,
};

use async_std::io::timeout;
use lxi_device::{lock::SharedLock, util::SimpleDevice};
use lxi_socket::{server::ServerConfig, SOCKET_STANDARD_PORT};

use clap::Parser;

use async_rustls::{
    rustls::{
        internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys},
        AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, Certificate,
        NoClientAuth, PrivateKey, RootCertStore, ServerConfig as TlsConfig,
    },
    TlsAcceptor,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = SOCKET_STANDARD_PORT)]
    port: u16,

    /// Kill server after timeout (useful for coverage testing)
    #[clap(short, long)]
    timeout: Option<u64>,

    /// TLS certificate
    #[clap(short, long, default_value = ".certificates/cert.pem")]
    cert: String,

    /// TLS key
    #[clap(short, long, default_value = ".certificates/key.pem")]
    key: String,

    #[clap(long)]
    client_cert: Vec<String>,

    #[clap(long)]
    require_authentication: bool,
}

/// Load the passed certificates file
fn load_certs(path: &str) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
}

/// Load the passed keys file
fn load_keys(path: &str) -> io::Result<Vec<PrivateKey>> {
    // Try to load RSA key
    match rsa_private_keys(&mut BufReader::new(File::open(path)?)) {
        Ok(keys) => Ok(keys),
        // Try PKCS#8 if not RSA
        Err(_) => match pkcs8_private_keys(&mut BufReader::new(File::open(path)?)) {
            Ok(keys) => Ok(keys),
            Err(_) => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid key, expected RSA or PKCS#8 in PEM format"))
            },
        },
    }
}

/// Configure the server using rusttls
/// See https://docs.rs/rustls/0.16.0/rustls/struct.ServerConfig.html for details
///
/// A TLS server needs a certificate and a fitting private key
fn load_config(options: &Args) -> io::Result<TlsConfig> {
    let certs = load_certs(&options.cert)?;
    let mut keys = load_keys(&options.key)?;

    let mut config = if !options.client_cert.is_empty() {
        let mut store = RootCertStore::empty();
        for path in &options.client_cert {
            let mut reader = BufReader::new(File::open(path)?);
            store
                .add_pem_file(&mut reader)
                .expect("Failed to load client certificate");
        }
        if options.require_authentication {
            TlsConfig::new(AllowAnyAuthenticatedClient::new(store))
        } else {
            TlsConfig::new(AllowAnyAnonymousOrAuthenticatedClient::new(store))
        }
    } else {
        if options.require_authentication {
            log::error!("Client authentication required but no certificates were provided")
        }
        TlsConfig::new(NoClientAuth::new())
    };

    config
        // set this server to use one cert together with the loaded private key
        .set_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    Ok(config)
}

#[async_std::main]
async fn main() -> std::io::Result<()> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device = SimpleDevice::new_arc();
    let shared_lock = SharedLock::new();

    // TLS
    let config = load_config(&args)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let ipv4_server = ServerConfig::default()
        .read_buffer(16 * 1024)
        .build()
        .accept_tls((&args.ip[..], args.port), shared_lock, device, acceptor);

    log::info!("Running server on port {}:{}...", args.ip, args.port);
    if let Some(t) = args.timeout {
        timeout(Duration::from_millis(t), ipv4_server).await
    } else {
        ipv4_server.await
    }
}
