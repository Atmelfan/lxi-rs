use alloc::{sync::Arc, vec::Vec};
use futures::lock::Mutex;

use crate::Device;

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

    fn get_status(&mut self) -> u8 {
        Ok(0)
    }

    fn trigger(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }
}

pub struct SimpleDevice;

impl SimpleDevice {
    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Device for SimpleDevice {
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8> {
        match cmd.as_slice() {
            x if x.eq_ignore_ascii_case(b"*IDN?") => b"Cyberdyne systems,T800 Model 101,A9012.C,V2.4".to_vec(),
            x if x.eq_ignore_ascii_case(b"EVENT") => b"".to_vec(),
            x if x.eq_ignore_ascii_case(b"QUERY?") => b"RESPONSE".to_vec(),
            _ => cmd.clone(),
        }
    }

    fn get_status(&mut self) -> u8 {
        0
    }

    fn trigger(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }
}
