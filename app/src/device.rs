use std::sync::Arc;

use lxi_device::{lock::Mutex, Device, DeviceError};

pub struct EchoDevice;

impl EchoDevice {
    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Device for EchoDevice {
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8> {
        cmd.clone()
    }

    fn get_status(&mut self) -> Result<u8, DeviceError> {
        Ok(0)
    }

    fn trigger(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn set_remote(&mut self, _remote: bool) -> Result<(), DeviceError> {
        Ok(())
    }
}
