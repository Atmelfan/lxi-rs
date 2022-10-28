use std::vec;

use tide::{Request, Response};

pub use crate::common::lxi::identification::*;

pub async fn get<S>(req: Request<S>) -> tide::Result
where
    S: Identification,
{
    let state = req.state();
    // Location of schema
    let mut schema = req.url().clone();
    schema.set_path("lxi/schemas/LXIIdentification/1.0");

    // Location of identification document (this?)
    let mut identification = req.url().clone();
    identification.set_path("lxi/identification");

    // Response xml
    let mut res: Response = LXIIdentification {
        xmlns: "http://www.lxistandard.org/InstrumentIdentification/1.0".to_string(),
        xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
        xsi_schema_location: format!(
            "http://www.lxistandard.org/InstrumentIdentification/1.0 {}",
            schema.as_str()
        ),
        manufacturer: state.manufacturer(),
        model: state.manufacturer(),
        serial_number: state.serial_number(),
        firmware_revision: state.firmware_revision(),
        manufacturer_description: state.manufacturer_description(),
        homepage_url: state.homepage_url(),
        driver_url: state.driver_url(),
        connected_devices: ConnectedDevices {
            devices: state
                .connected_devices()
                .iter()
                .map(|s| DeviceUri {
                    device_uri: {
                        // Location of identification document (this?)
                        let mut url = req.url().clone();
                        url.set_path(format!("/devices/{s}/").as_str());
                        url.to_string()
                    },
                })
                .collect(),
        },
        user_description: state.user_description(),
        identification_url: identification.to_string(),
        interfaces: state.interfaces(),
        ivisoftware_module_name: state.ivisoftware_module_name(),
        domain: state.domain(),
        lxi_version: <S as Identification>::lxi_version(),
        extended_functions: ExtendedFunctions {
            extended_functions: state.extended_functions(),
        },
    }
    .to_xml()?
    .into();
    res.set_content_type("text/xml");
    Ok(res)
}

pub trait Identification {
    fn lxi_version() -> String;

    /// Information about the device
    fn manufacturer(&self) -> String;

    fn model(&self) -> String;

    fn serial_number(&self) -> String;

    fn firmware_revision(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn manufacturer_description(&self) -> String {
        env!("CARGO_PKG_DESCRIPTION").to_string()
    }

    fn homepage_url(&self) -> String {
        env!("CARGO_PKG_HOMEPAGE").to_string()
    }

    fn driver_url(&self) -> String {
        env!("CARGO_PKG_HOMEPAGE").to_string()
    }

    fn ivisoftware_module_name(&self) -> Vec<IVISoftwareModuleName> {
        vec![]
    }

    /// List implemented extended functions
    fn extended_functions(&self) -> Vec<Function> {
        vec![]
    }

    /// List attached interfaces
    fn interfaces(&self) -> Vec<Interface>;

    /// User description. Should as the name suggest be configurable by the user.
    fn user_description(&self) -> String;

    /// Domain
    fn domain(&self) -> Option<u8> {
        None
    }

    /// Host address for this device. Can be the IP address, hostname, hostname.local, etc...
    fn host(&self) -> String;

    fn connected_devices(&self) -> Vec<String> {
        vec![]
    }
}
