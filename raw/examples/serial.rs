use std::time::Duration;

use async_io::Async;
use async_std::io::timeout;
use futures::AsyncReadExt;
use lxi_device::{lock::SharedLock, util::SimpleDevice};
use lxi_socket::server::ServerConfig;

use clap::Parser;
use mio_serial::SerialPortBuilderExt;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "/dev/ttyS0")]
    path: String,

    #[clap(short, long, default_value_t = 9600)]
    baudrate: u32,

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

    let (reader, writer) = mio_serial::new(&args.path, args.baudrate)
        .open_native_async()
        .map_err(|e| e.into())
        .and_then(|port| Async::new(port))
        .expect("Failed to open port")
        .split();

    let serial_server = ServerConfig::default()
        .read_buffer(16 * 1024)
        .build()
        .process_client(reader, writer, shared_lock, device, &args.path);

    log::info!("Running server on port {}...", args.path);
    if let Some(t) = args.timeout {
        timeout(Duration::from_millis(t), serial_server).await
    } else {
        serial_server.await
    }
}
