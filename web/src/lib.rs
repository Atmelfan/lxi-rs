use lxi_device::lock::LockHandle;
use sqlx::SqlitePool;

pub mod records;
pub mod routes;
pub mod templates;
pub mod utils;

#[cfg(feature = "websockets")]
pub mod websockets;

pub type Request = tide::Request<State>;

#[derive(Clone)]
pub struct State {
    pub db: SqlitePool,
}

pub struct LxiDeviceInfo {
    model: String,
    manufacturer: String,
    serial_number: String,
    version: String,
    hostname: String,
}

pub trait LxiState {
    fn get_device_info(&self) -> LxiDeviceInfo;

    fn set_hostname(&self, hostname: &str);

    fn advertise_hislip(&self) -> bool;
}
