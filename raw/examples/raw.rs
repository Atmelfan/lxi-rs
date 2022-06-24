use embedded_nal_async::{TcpClientStack, TcpFullStack};
use futures::executor::LocalPool;
use lxi_device::{lock::SharedLock, util::SimpleDevice};
use lxi_socket::{server::ServerConfig, SOCKET_STANDARD_PORT};

use embedded_nal_async_std::Stack;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    ip: Option<String>,

    #[clap(short, long, default_value_t = SOCKET_STANDARD_PORT)]
    port: u16,
}

#[async_std::main]
async fn main() -> Result<(), <Stack as TcpClientStack>::Error> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device0 = SimpleDevice::new_arc();
    let shared_lock0 = SharedLock::new();

    // Bind multiple devices for instruments with multiple sub-addresses like inst0, inst1 etc
    //let device1 = SimpleDevice::new_arc();
    //let shared_lock1 = SharedLock::new();

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    let stack = if let Some(ip) = args.ip {
        Stack::new(ip.parse().expect("Invalid ip address"))
    } else {
        Stack::default()
    };

    let server = ServerConfig::default().read_buffer(16 * 1024).build();
    let socket0 = server.bind(stack.clone(), args.port, shared_lock0, device0, spawner);
    //let socket1 = server.bind(stack.clone(), args.port + 1, shared_lock2, device2, spawner);

    log::info!("Running server on port {}", args.port);
    pool.run_until(socket0)
    //pool.run_until(async move {
    //    futures::try_join!(socket0, socket1)?;
    //    Ok(())
    //})
}
