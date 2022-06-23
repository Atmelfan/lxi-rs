use std::{io, net::Ipv4Addr, sync::Arc};

use lxi_vxi11::server::{portmapper::prelude::*, vxi11::prelude::*};

use clap::Parser;

/// Demo VXI-11 server using system rpcbind/portmap
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Ip for server to bind to
    #[clap(default_value = "127.0.0.1")]
    ip: String,

    #[clap(default_value_t = PORTMAPPER_PORT)]
    port: u16,

    /// Port of Core channel
    #[clap(default_value_t = 4322)]
    core_port: u16,

    /// Port of Async channel
    #[clap(default_value_t = 4323)]
    async_port: u16,
}

#[async_std::main]
async fn main() -> io::Result<()> {
    femme::start();
    let args = Args::parse();

    println!("Running server ...");
    let portmap = StaticPortMap::new([
        Mapping::new(
            DEVICE_CORE, // VXI-11 CORE
            DEVICE_CORE_VERSION,
            PORTMAPPER_PROT_TCP,
            args.core_port as u32,
        ),
        Mapping::new(
            DEVICE_ASYNC, // VXI-11 ASYNC
            DEVICE_ASYNC_VERSION,
            PORTMAPPER_PROT_TCP,
            args.async_port as u32,
        ),
    ]);
    portmap.bind((Ipv4Addr::UNSPECIFIED, args.port)).await
}
