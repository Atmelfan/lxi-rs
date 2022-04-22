use alloc::{
    vec::Vec,
    sync::Arc
};
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
        0
    }
}