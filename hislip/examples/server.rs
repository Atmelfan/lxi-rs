use async_std::io;
use lxi_device::{lock::SharedLock, util::{EchoDevice, SimpleDevice}};
use lxi_hislip::server::Server;
pub use lxi_hislip::{PROTOCOL_2_0, STANDARD_PORT};

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "127.0.0.1")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = STANDARD_PORT)]
    port: u16,
}

#[async_std::main]
async fn main() -> Result<(), io::Error> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device = SimpleDevice::new_arc();
    let shared_lock = SharedLock::new();

    let server = Server::new(0x1234, shared_lock, device).accept((&args.ip[..], args.port));
    println!("Running server on port {}:{}...", args.ip, args.port);
    server.await
}
