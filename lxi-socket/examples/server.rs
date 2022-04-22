use lxi_device::{util::EchoDevice, lock::SharedLock};
use lxi_socket::{common::SOCKET_PORT, server::ServerConfig};

#[async_std::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let device = EchoDevice::new_arc();
    let shared_lock = SharedLock::new();

    let ipv4_server = ServerConfig::new()
        .read_buffer(16 * 1024)
        .write_buffer(16 * 1024)
        .build()
        .serve(("127.0.0.1", SOCKET_PORT), shared_lock, device);

    println!("Running server...");
    ipv4_server.await
}
