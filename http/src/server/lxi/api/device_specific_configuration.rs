use tide::Request;

use crate::common::lxi::api::device_specific_configuration::*;

use super::auth::Permission;

pub trait DeviceSpecificConfiguration {
    /// Reset LAN config to LCI presets
    fn lan_config_initialize(&self);
}

pub async fn get<S>(req: Request<S>) -> tide::Result
where
    S: DeviceSpecificConfiguration,
{
    let _permissions = req.ext::<Permission>();
    todo!()
}

pub async fn put<S>(req: Request<S>) -> tide::Result
where
    S: DeviceSpecificConfiguration,
{
    let _permissions = req.ext::<Permission>();
    todo!()
}
