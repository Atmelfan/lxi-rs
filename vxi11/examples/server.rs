use lxi_device::{util::EchoDevice, lock::SharedLock};

#[async_std::main]
async fn main() {
    let device = EchoDevice::new_arc();
    let shared_lock = SharedLock::new();

    
}