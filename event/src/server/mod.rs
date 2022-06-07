use std::sync::Arc;

use futures::lock::Mutex;



struct Server<DEV> {
    device: Arc<Mutex<DEV>>,
    // Domain to listen to
    domain: u8
}

impl<DEV> Server<DEV> where DEV: lxi_device::Device {
    pub(crate) async fn trigger(&self, source: lxi_device::trigger::Source) -> Result<(), lxi_device::DeviceError> {
        let mut dev = self.device.lock().await;
        dev.trigger(source)
    }



}



