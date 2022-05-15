use alloc::{sync::Arc, vec::Vec};
use futures::lock::Mutex;

use crate::{Device, DeviceError};

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

pub struct SimpleDevice {
    trig: bool,
    rmt: bool
}

impl SimpleDevice {
    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            rmt: false,
            trig: false,
        }))
    }
}

impl Device for SimpleDevice {
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8> {
        log::info!(">>> {:?}", cmd);
        let r = match cmd.as_slice() {
            x if x.eq_ignore_ascii_case(b"*IDN?\n") => {
                b"Cyberdyne systems,T800 Model 101,A9012.C,V2.4\n".to_vec()
            }
            x if x.eq_ignore_ascii_case(b"EVENT\n") => b"".to_vec(),
            x if x.eq_ignore_ascii_case(b"QUERY?\n") => b"RESPONSE\n".to_vec(),
            _ => cmd.clone(),
        };
        log::info!("<<< {:?}", r);
        r
    }

    fn get_status(&mut self) -> Result<u8, DeviceError> {
        let mut stb = 0;
        stb |= (self.rmt as u8) << 7;
        stb |= (self.trig as u8) << 6;
        log::info!("STATUS = {}", stb);
        Ok(stb)
    }

    fn trigger(&mut self) -> Result<(), DeviceError> {
        log::info!("TRIGGERED");
        self.trig = true;
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        log::info!("CLEAR");
        self.trig = false;
        Ok(())
    }

    fn set_remote(&mut self, remote: bool) -> Result<(), DeviceError> {
        log::info!("REMOTE = {}", remote);
        self.rmt = remote;
        Ok(())
    }
}
