use sqlx::SqlitePool;


pub mod utils;
pub mod templates;
pub mod routes;
pub mod records;

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