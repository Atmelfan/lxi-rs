use alloc::{sync::Arc, vec::Vec};
use futures::lock::Mutex;

use crate::{Device, DeviceError, trigger::Source};

/// A device that echoes any command sent to it.
#[derive(Clone)]
pub struct EchoDevice;

impl EchoDevice {
    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Device for EchoDevice {
    fn execute(&mut self, cmd: &[u8]) -> Option<Vec<u8>> {
        Some(cmd.to_vec())
    }

    fn get_status(&mut self) -> Result<u8, DeviceError> {
        Ok(0)
    }

    fn trigger(&mut self, _: Source) -> Result<(), DeviceError> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn set_remote(&mut self, _remote: bool) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// A device with some simple commands like `*IDN?`, `EVENT` and `QUERY?`.
/// Useful for debugging
#[derive(Clone)]
pub struct SimpleDevice {
    trig: bool,
    clear: bool,
    rmt: bool,
}

impl SimpleDevice {
    pub fn new() -> Self {
        Self {
            rmt: false,
            trig: false,
            clear: false,
        }
    }

    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }
}

impl Device for SimpleDevice {
    fn execute(&mut self, cmd: &[u8]) -> Option<Vec<u8>> {
        log::debug!(">>> {:?}", cmd);
        let r = match cmd {
            x if x.eq_ignore_ascii_case(b"*IDN?") || x.eq_ignore_ascii_case(b"*IDN?\n") => {
                Some(b"Cyberdyne systems,T800 Model 101,A9012.C,V2.4".to_vec())
            }
            x if x.eq_ignore_ascii_case(b"EVENT") || x.eq_ignore_ascii_case(b"EVENT\n") => {
                None
            }
            x if x.eq_ignore_ascii_case(b"QUERY?") || x.eq_ignore_ascii_case(b"QUERY?\n") => {
                Some(b"RESPONSE".to_vec())
            }
            _ => {
                let mut rev = cmd.to_vec();
                rev.reverse();
                Some(rev)
            }
        };
        log::debug!("<<< {:?}", r);
        r
    }

    fn get_status(&mut self) -> Result<u8, DeviceError> {
        let mut stb = 0;
        stb |= (self.rmt as u8) << 7;
        stb |= (self.trig as u8) << 6;
        stb |= (self.clear as u8) << 5;
        log::info!("===== STATUS={} =====", stb);
        Ok(stb)
    }

    fn trigger(&mut self, source: Source) -> Result<(), DeviceError> {
        log::info!("===== TRIGGERED BY {source:?} =====");
        self.trig = true;
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        log::info!("===== CLEAR =====");
        self.trig = false;
        self.clear = true;
        Ok(())
    }

    fn set_remote(&mut self, remote: bool) -> Result<(), DeviceError> {
        log::info!("===== REMOTE={} =====", remote);
        self.rmt = remote;
        Ok(())
    }
}
