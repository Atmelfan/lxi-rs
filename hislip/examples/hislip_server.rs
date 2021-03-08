use std::any::Any;
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_std::io;
use async_std::prelude::*;
use async_std::sync::Mutex;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs}, // 3
    prelude::*,                                   // 1
    task,                                         // 2
};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use rustls::{Certificate, NoClientAuth, PrivateKey, ServerConfig};
use structopt::StructOpt;

use hislip::protocol::errors::{Error, FatalErrorCode, NonFatalErrorCode};
pub use hislip::protocol::messages::Protocol;
use hislip::protocol::messages::{Header, MessageType};
use hislip::server::Server;
pub use hislip::PROTOCOL_2_0;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(StructOpt)]
struct Options {
    addr: String,

    /// cert file
    #[structopt(short = "c", long = "cert", parse(from_os_str))]
    cert: PathBuf,

    /// key file
    #[structopt(short = "k", long = "key", parse(from_os_str))]
    key: PathBuf,
}

/// Load the passed certificates file
fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
}

/// Load the passed keys file
fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
}

/// Configure the server using rusttls
/// See https://docs.rs/rustls/0.16.0/rustls/struct.ServerConfig.html for details
///
/// A TLS server needs a certificate and a fitting private key
fn load_config(options: &Options) -> io::Result<ServerConfig> {
    let certs = load_certs(&options.cert)?;
    let mut keys = load_keys(&options.key)?;

    // we don't use client authentication
    let mut config = ServerConfig::new(NoClientAuth::new());
    config
        // set this server to use one cert together with the loaded private key
        .set_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    Ok(config)
}

enum SecureConnection {
    /// Secure connection is not supported
    NotSupported,
    #[cfg(feature = "secure-connection")]
    /// Secure connection is supported but optional
    Supported,
    #[cfg(feature = "secure-connection")]
    /// Secure connection is supported and mandatory
    Mandatory,
}

fn main() -> Result<()> {
    env_logger::init();

    let server = Server::new(0x1234);
    task::block_on(server.accept("127.0.0.1:4880"))
}
