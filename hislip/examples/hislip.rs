use std::{sync::Arc, time::Duration};

use async_std::{
    io::{self, timeout},
    task,
};
use futures::{lock::Mutex, task::Spawn};
use lxi_device::{
    lock::SharedLock,
    status::Sender as StatusSender,
    util::{EchoDevice, SimpleDevice},
    Device,
};
use lxi_hislip::{
    server::{ServerBuilder, ServerConfig},
    STANDARD_PORT,
};

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(default_value = "0.0.0.0")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = STANDARD_PORT)]
    port: u16,

    /// Kill server after timeout (useful for coverage testing)
    #[clap(short, long)]
    timeout: Option<u64>,
}

struct DummySpawner;
impl Spawn for DummySpawner {
    fn spawn_obj(
        &self,
        future: futures::future::FutureObj<'static, ()>,
    ) -> Result<(), futures::task::SpawnError> {
        task::spawn(async move { future.await });
        Ok(())
    }
}

#[async_std::main]
async fn main() -> Result<(), io::Error> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Args::parse();

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

    let shared_lock0 = SharedLock::new();
    let device0: Arc<Mutex<Box<dyn Device + Send>>> =
        Arc::new(Mutex::new(Box::new(SimpleDevice::new())));

    let shared_lock1 = SharedLock::new();
    let device1: Arc<Mutex<Box<dyn Device + Send>>> = Arc::new(Mutex::new(Box::new(EchoDevice)));

    let config = ServerConfig::default().vendor_id(0x1234);
    //.short_idn(b"GPA-Robotics,hislip-demo,0,0");
    let server = ServerBuilder::new(config)
        .device("hislip0".to_string(), device0, shared_lock0)
        .device("hislip1".to_string(), device1, shared_lock1)
        .build();

    log::info!("Running server on port {}:{}...", args.ip, args.port);
    if let Some(t) = args.timeout {
        timeout(
            Duration::from_millis(t),
            server.accept((&args.ip[..], args.port), srq.clone(), DummySpawner),
        )
        .await
    } else {
        server
            .accept((&args.ip[..], args.port), srq.clone(), DummySpawner)
            .await
    }
}
