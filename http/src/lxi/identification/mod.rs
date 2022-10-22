use tide::{Request, Response};

use self::{
    functions::Function,
    xml::{ConnectedDevices, DeviceUri, ExtendedFunctions, Identification, Interface},
};

pub mod functions;
pub mod xml;

pub async fn handler<S>(req: Request<S>) -> tide::Result
where
    S: IdentityProvider,
{
    let url = req.url();
    let mut res: Response = req
        .state()
        .get_identification(req.host(), url.scheme())
        .to_xml()?
        .into();
    res.set_content_type("application/xml");
    Ok(res)
}

pub trait IdentityProvider {
    fn lxi_version() -> String;

    /// Information about the device
    fn manufacturer(&self) -> String;
    fn model(&self) -> String;
    fn serial_number(&self) -> String;
    fn firmware_revision(&self) -> String;
    fn manufacturer_description(&self) -> String;
    fn homepage_url(&self) -> String;
    fn driver_url(&self) -> String;

    fn ivisoftware_module_name(&self) -> Option<String> {
        None
    }

    fn extended_functions(&self) -> Vec<Function>;
    fn interfaces(&self) -> Vec<Interface>;

    /// User configurable
    fn user_description(&self) -> String;
    fn domain(&self) -> Option<u8> {
        None
    }

    /// Host address for this device. Can be the IP address, hostname, hostname.local, etc...
    fn host(&self) -> String;

    fn connected_devices(&self) -> Option<Vec<String>> {
        Some(vec![
            "devices/device0/".to_string(),
            "devices/device1/".to_string(),
        ])
    }

    fn get_identification(&self, host: Option<&str>, scheme: &str) -> Identification {
        let backup = self.host();
        let host = host.unwrap_or(&backup);
        Identification {
            xmlns: "http://www.lxistandard.org/InstrumentIdentification/1.0".to_string(),
            xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
            xsi_schema_location:
                "http://www.lxistandard.org/InstrumentIdentification/1.0/LXIIdentification.xsd"
                    .to_string(),
            manufacturer: self.manufacturer(),
            model: self.manufacturer(),
            serial_number: self.serial_number(),
            firmware_revision: self.firmware_revision(),
            manufacturer_description: self.manufacturer_description(),
            homepage_url: self.homepage_url(),
            driver_url: self.driver_url(),
            connected_devices: self.connected_devices().map(|devices| ConnectedDevices {
                devices: devices
                    .iter()
                    .map(|s| DeviceUri {
                        device_uri: format!("{scheme}://{host}/{s}"),
                    })
                    .collect(),
            }),
            user_description: self.user_description(),
            identification_url: format!("{scheme}://{host}/lxi/identification"),
            interfaces: self.interfaces(),
            ivisoftware_module_name: self.ivisoftware_module_name(),
            domain: self.domain(),
            lxi_version: Self::lxi_version(),
            extended_functions: ExtendedFunctions {
                extended_functions: self.extended_functions(),
            },
        }
    }
}
