use std::{
    io,
    net::{IpAddr, Ipv4Addr},
};

use futures::try_join;
use lxi_device::{lock::SharedLock, util::EchoDevice};
use vxi11::server::{portmapper::PORTMAPPER_PORT, vxi11::VxiServerBuilder};

use clap::Parser;

/// Demo VXI-11 server using system rpcbind/portmap
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Ip for server to bind to
    #[clap(default_value = "127.0.0.1")]
    ip: String,

    /// Port of Core channel
    #[clap(default_value_t = 4322)]
    core_port: u16,

    /// Port of Async channel
    #[clap(default_value_t = 4323)]
    async_port: u16,

    /// Register using system rpcbind/portmap
    #[clap(short, long)]
    register: bool,
}

#[async_std::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let device = EchoDevice::new_arc();
    let shared = SharedLock::new();

    let (vxi11_core, vxi11_async) = if args.register {
        VxiServerBuilder::new()
            .core_port(args.core_port)
            .async_port(args.async_port)
            .register_portmap((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT))
            .await
            .map_err(|e| log::error!("Failed to register with rpcbind/portmap: {:?}", e))
            .unwrap()
    } else {
        VxiServerBuilder::new()
    }
    .build(shared, device);

    println!("Running server ...");
    try_join!(
        vxi11_core.serve(args.ip.parse().unwrap()),
        vxi11_async.serve(args.ip.parse().unwrap())
    )
    .map(|_| ())
}
