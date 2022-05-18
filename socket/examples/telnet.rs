use lxi_device::{lock::SharedLock, util::SimpleDevice};
use lxi_socket::{TELNET_STANDARD_PORT, server::telnet::ServerConfig};

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = TELNET_STANDARD_PORT)]
    port: u16,
}

#[async_std::main]
async fn main() -> std::io::Result<()> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device = SimpleDevice::new_arc();
    let shared_lock = SharedLock::new();

    let ipv4_server = ServerConfig::default()
        .read_buffer(16 * 1024)
        .build()
        .accept((&args.ip[..], args.port), shared_lock, device);

    println!("Running server on port {}:{}...", args.ip, args.port);
    ipv4_server.await
}
