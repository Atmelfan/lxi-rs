use std::{io, net::Ipv4Addr, time::Duration};

use async_std::{
    future::timeout,
    net::{TcpListener, TcpStream},
    task,
};
use futures::try_join;
use lxi_device::{
    lock::SharedLock,
    status::Sender as StatusSender,
    util::{EchoDevice, SimpleDevice},
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
}

#[async_std::main]
async fn main() -> io::Result<()> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

    let device = SimpleDevice::new_arc();
    let shared = SharedLock::new();

    let core_listener = TcpListener::bind(args.core_addr).await?;
    let async_listener = TcpListener::bind(args.async_addr).await?;

    let srq = StatusSender::new();

    // Spam service requests
    let mut srq_spammer = srq.clone(); 
    task::spawn(async move {
        task::sleep(Duration::from_secs(10)).await;
        srq_spammer.send_status(0);
    });

    let (vxi11_core, vxi11_async) = VxiServerBuilder::new()
        .core_port(core_listener.local_addr()?.port())
        .async_port(async_listener.local_addr()?.port())
        .build(shared, device, srq);

    // Kill server after 10s
    let kill_timeout = async {
        log::warn!("Will kill server after 10s");
        task::sleep(Duration::from_millis(10000)).await;
        log::warn!("Killing server...");
        Err(io::Error::from(io::ErrorKind::TimedOut))?;
        Ok(())
    };

    if args.register {
        let mut portmap =
            PortMapperClient::connect_tcp((Ipv4Addr::LOCALHOST, PORTMAPPER_PORT)).await?;

        // Register core service
        let _core_unset = portmap
            .unset(Mapping::new(
                DEVICE_CORE,
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                0,
            ))
            .await
            .expect("Failed to unset core channel");
        let core_set = portmap
            .set(Mapping::new(
                DEVICE_CORE,
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                core_listener.local_addr()?.port() as u32,
            ))
            .await
            .expect("Failed to register core channel");
        log::info!("portmap::set(DEVICE_CORE) returned {}", core_set);

        // Register async service
        let _async_unset = portmap
            .unset(Mapping::new(
                DEVICE_ASYNC,
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                0,
            ))
            .await
            .expect("Failed to unset async channel");
        let async_set = portmap
            .set(Mapping::new(
                DEVICE_ASYNC,
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                async_listener.local_addr()?.port() as u32,
            ))
            .await
            .expect("Failed to register async channel");
        log::info!("portmap::set(DEVICE_ASYNC) returned {}", async_set);

        println!("Running server ...");
        try_join!(
            kill_timeout,
            vxi11_core.serve(core_listener),
            vxi11_async.serve(async_listener)
        )
        .map(|_| ())
    } else {
        let portmap = StaticPortMapBuilder::new()
            .set(Mapping::new(
                DEVICE_CORE, // VXI-11 CORE
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                core_listener.local_addr()?.port() as u32,
            ))
            .set(Mapping::new(
                DEVICE_ASYNC, // VXI-11 ASYNC
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                async_listener.local_addr()?.port() as u32,
            ))
            .build();

        println!("Running server ...");

        try_join!(
            kill_timeout,
            portmap.bind((Ipv4Addr::UNSPECIFIED, PORTMAPPER_PORT)),
            vxi11_core.serve(core_listener),
            vxi11_async.serve(async_listener)
        )
        .map(|_| ())
    }
}
