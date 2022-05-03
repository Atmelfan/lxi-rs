use async_std::io;
use lxi_device::{util::EchoDevice, lock::SharedLock};
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
    env_logger::init();
    let args = Args::parse();

    let device = EchoDevice::new_arc();
    let shared_lock = SharedLock::new();

    let server = Server::new(0x1234, shared_lock, device).accept((&args.ip[..], args.port));
    println!("Running server on port {}:{}...", args.ip, args.port);
    server.await
}
