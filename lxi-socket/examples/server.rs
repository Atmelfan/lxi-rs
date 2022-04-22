use lxi_device::{lock::SharedLock, util::EchoDevice};
use lxi_socket::{common::SOCKET_PORT, server::ServerConfig};

#[async_std::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let device = EchoDevice::new_arc();
    let shared_lock = SharedLock::new();

    let ipv4_server = ServerConfig::default()
        .read_buffer(16 * 1024)
        .build()
        .serve(("127.0.0.1", SOCKET_PORT), shared_lock, device);

    println!("Running server...");
    ipv4_server.await
}
