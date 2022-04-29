use async_std::io;

use async_std::task;

use hislip::server::Server;
pub use hislip::PROTOCOL_2_0;


fn main() -> Result<(), io::Error> {
    env_logger::init();

    let server = Server::new(0x1234);
    task::block_on(server.accept("127.0.0.1:4880"))
}
