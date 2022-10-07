use std::time::Duration;

use async_std::io::timeout;
use lxi_device::{lock::SharedLock, util::SimpleDevice};
use lxi_socket::{server::ServerConfig, SOCKET_STANDARD_PORT};

use clap::Parser;

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

    log::info!("Running server on port {}:{}...", args.ip, args.port);
    if let Some(t) = args.timeout {
        timeout(
            Duration::from_millis(t),
            ipv4_server
        )
        .await
    } else {
        ipv4_server.await
    }
    
}
