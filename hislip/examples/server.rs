use std::{path::{Path, PathBuf}, io::BufReader, fs::File, sync::Arc};

use async_std::io;
use lxi_device::{
    lock::SharedLock,
    util::{EchoDevice, SimpleDevice},
};
use lxi_hislip::server::Server;
pub use lxi_hislip::{PROTOCOL_2_0, STANDARD_PORT};

use clap::Parser;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;

#[cfg(feature = "tls")]
use rustls::{
    internal::pemfile::{certs, rsa_private_keys},
    Certificate, NoClientAuth, PrivateKey, ServerConfig,
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

#[async_std::main]
async fn main() -> Result<(), io::Error> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device = SimpleDevice::new_arc();
    let shared_lock = SharedLock::new();

    #[cfg(feature = "tls")]
    let acceptor = { 
        let config = load_config(&args)?;
        Arc::new(TlsAcceptor::from(Arc::new(config)))
    };

    let server = Server::new(0x1234, shared_lock, device).accept((&args.ip[..], args.port), #[cfg(feature = "tls")] acceptor);
    println!("Running server on port {}:{}...", args.ip, args.port);
    server.await
}
