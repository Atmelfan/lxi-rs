use lxi_device::{Device, SharedLock};
use lxi_socket::{common::SOCKET_PORT, server::TcpServerBuilder};

use async_std::sync::{Arc, Mutex};

struct TestDevice {}

impl Device for TestDevice {
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8> {
        todo!()
    }
}

fn main() -> std::io::Result<()> {
    let device = Arc::new(Mutex::new(TestDevice {}));
    let shared_lock = SharedLock::new();

    let ipv4_server = TcpServerBuilder::new(("127.0.0.1", SOCKET_PORT))
        .read_buffer(16 * 1024)
        .write_buffer(16 * 1024)
        .serve(shared_lock, device);

    async_std::task::block_on(ipv4_server)
}
