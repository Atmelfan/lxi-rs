mod device;

use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_std::io;
use lxi_device::{
    lock::SharedLock,
    util::{EchoDevice, SimpleDevice},
};
use lxi_hislip::server::Server as HislipServer;
use lxi_hislip::{PROTOCOL_2_0, STANDARD_PORT};

use lxi_socket::server::{Server as SocketServer, ServerConfig as SocketServerConfig};

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
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    /// cert file
    #[clap(short, long)]
    cert: PathBuf,

    /// key file
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

    // Start HiSLIP server
    let hislip_addr = args.ip.clone();
    let hislip_lock = shared_lock.clone();
    let hislip_device = device.clone();
    async_std::task::spawn(async move {
        let server = HislipServer::new(0x1234, hislip_lock, hislip_device);
        server
            .accept(
                (&hislip_addr[..], STANDARD_PORT),
                #[cfg(feature = "tls")]
                acceptor,
            )
            .await
    });

    // Start socket server
    let socket_addr = args.ip.clone();
    async_std::task::spawn(async move {
        let server = SocketServerConfig::default().read_buffer(16 * 1024).build();
        server
            .accept((&socket_addr[..], STANDARD_PORT), shared_lock, device)
            .await
    });

    Ok(())
}
