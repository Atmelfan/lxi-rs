use lxi_device::{lock::SharedLock, util::EchoDevice};
use lxi_socket::{common::SOCKET_PORT, server::ServerConfig};

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "127.0.0.1")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = SOCKET_PORT)]
    port: u16,
}


#[async_std::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let args = Args::parse();
    
    let device = EchoDevice::new_arc();
    let shared_lock = SharedLock::new();

    let ipv4_server = ServerConfig::default()
        .read_buffer(16 * 1024)
        .build()
        .serve((&args.ip[..], args.port), shared_lock, device);

    println!("Running server on port {}:{}...", args.ip, args.port);
    ipv4_server.await
}