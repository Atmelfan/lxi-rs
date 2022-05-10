use std::sync::Arc;

use async_std::{io::BufReader, os::unix::net::UnixStream};
use futures::{join, lock::Mutex, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use lxi_device::{
    lock::{SharedLock, SpinMutex},
    util::EchoDevice,
};
use lxi_socket::server::ServerConfig;

async fn run_echo_server(
    stream: UnixStream,
    shared_lock: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<EchoDevice>>,
) {
    let peer = stream.peer_addr().unwrap();
    let (reader, writer) = stream.split();
    let server = ServerConfig::default().read_buffer(16 * 1024).build();
    println!("Running server...");
    let ret = server
        .process_client(reader, writer, shared_lock, device, peer)
        .await;
    println!("Server exit {:?}", ret);
    assert!(ret.is_ok());
}

#[async_std::test]
async fn echo() {
    env_logger::init();

    let device = EchoDevice::new_arc();
    let shared_lock = SharedLock::new();

    // Create
    let (client_stream, server_stream) = async_std::os::unix::net::UnixStream::pair().unwrap();

    let client_fut = async move {
        let (client_read, mut client_write) = client_stream.split();
        let mut client_read = BufReader::new(client_read);
        let mut buf = Vec::new();

        println!("Running client...");

        client_write.write_all(b"test\n").await.unwrap();

        client_read.read_until(b'\n', &mut buf).await.unwrap();
        assert_eq!(buf.as_slice(), b"test\n");
    };

    join!(
        run_echo_server(server_stream, shared_lock, device),
        client_fut
    );
}
