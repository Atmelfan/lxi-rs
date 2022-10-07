use std::{io, net::Ipv4Addr, time::Duration};

use async_std::{
    future::pending,
    net::TcpListener,
    task::{self, spawn},
};
use futures::{try_join, FutureExt};
use lxi_device::{
    lock::SharedLock,
    status::Sender as StatusSender,
    util::SimpleDevice,
};
use lxi_vxi11::{
    client::portmapper::prelude::*,
    server::{portmapper::prelude::*, vxi11::prelude::*},
};

use clap::Parser;

/// Demo VXI-11 server using system rpcbind/portmap
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address of Core channel
    #[clap(default_value = "127.0.0.1:0")]
    core_addr: String,

    /// Address of Async channel
    #[clap(default_value = "127.0.0.1:0")]
    async_addr: String,

    /// Register using system rpcbind/portmap
    #[clap(short, long)]
    register: bool,

    #[clap(short, long)]
    timeout: Option<u64>,
}

#[async_std::main]
async fn main() -> io::Result<()> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device = SimpleDevice::new_arc();
    let shared = SharedLock::new();

    let core_listener = TcpListener::bind(args.core_addr).await?;
    let core_port = core_listener.local_addr()?.port();
    let async_listener = TcpListener::bind(args.async_addr).await?;
    let async_port = async_listener.local_addr()?.port();

    let srq = StatusSender::new();

    // Spam service requests
    let mut srq_spammer = srq.clone();
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(10)).await;
            log::info!("Sending srq!");
            srq_spammer.send_status(0);
        }
    });

    // Kill server after 10s
    let timeout = match args.timeout {
        Some(t) => {
            log::warn!("Will kill server after 10s");
            async move {
                task::sleep(Duration::from_millis(t)).await;
                log::warn!("Killing server...");
                Err::<(), async_std::io::Error>(async_std::io::ErrorKind::TimedOut.into())
            }
            .right_future()
        }
        None => pending().left_future(),
    };

    let (vxi11_core, vxi11_async) = VxiServerBuilder::new()
        .core_port(core_listener.local_addr()?.port())
        .async_port(async_listener.local_addr()?.port())
        .build(shared, device, srq);

    if args.register {
        let mut portmap =
            PortMapperClient::connect_tcp((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT)).await?;

        // Register core service
        portmap
            .register(Mapping::new(
                DEVICE_CORE,
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                core_port as u32,
            ))
            .await
            .expect("Failed to register core channel");

        // Register async service
        portmap
            .register(Mapping::new(
                DEVICE_ASYNC,
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                async_port as u32,
            ))
            .await
            .expect("Failed to register async channel");
    } else {
        let portmap = StaticPortMap::new([
            Mapping::new(
                DEVICE_CORE, // VXI-11 CORE
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                core_listener.local_addr()?.port() as u32,
            ),
            Mapping::new(
                DEVICE_ASYNC, // VXI-11 ASYNC
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                async_listener.local_addr()?.port() as u32,
            )]);

        log::info!("Running portmap ...");
        spawn(async move {
            portmap
                .bind((Ipv4Addr::UNSPECIFIED, PORTMAPPER_PORT))
                .await
                .expect("Failed to run portmap")
        });
    }

    let core_handle = spawn(vxi11_core.serve(core_listener));
    let async_handle = spawn(vxi11_async.serve(async_listener));

    log::info!("Running server ...");
    try_join!(core_handle, async_handle, timeout).map(|_| ())
}
