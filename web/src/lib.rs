use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use lxi_device::lock::LockHandle;
use sqlx::SqlitePool;
use xml::{
    writer::{Error as EmitterError, XmlEvent},
    EmitterConfig, EventWriter,
};

pub mod records;
pub mod routes;
pub mod templates;
pub mod utils;

pub type Request = tide::Request<State>;

pub fn new<S>(state: S, inst: &str) -> tide::Server<S> 
where 
    S: LxiState + Clone + Send + Sync + 'static, {
    let mut app = tide::with_state(state);

    app
}

#[derive(Clone)]
pub struct State {
    pub db: SqlitePool,
}

impl LxiState for State {
    fn get_device_info(&self) -> DeviceInfo {
        DeviceInfo::new()
    }

    fn get_lxi_info(&self) -> LxiInfo {
        LxiInfo::new(
            "Thingimajing".to_string(),
            "1".to_string(),
            "1.5".to_string(),
        )
    }

    fn set_hostname(&self, hostname: &str) {
        todo!()
    }

    fn get_interfaces(&self) -> Vec<Interface> {
        vec![
            Interface {
                typ: "LXI".to_string(),
                name: "eth0".to_string(),
                instr_addr_string: vec![
                    "TCPIP::sampledevice.local::inst0::INSTR".to_string(),
                    "TCPIP::sampledevice.local::hislip0::INSTR".to_string(),
                    "TCPIP::sampledevice.local::5025::SOCKET".to_string(),
                ],
                network_info: Some(NetworkInformation {
                    hostname: "sampledevice.local".to_string(),
                    ip_addr: Ipv4Addr::new(192, 168, 10, 249).into(),
                    subnet_mask: "255.255.255.0".to_string(),
                    mac_address: "00:bb:60:7e:c8:9a".to_string(),
                    gateway: Ipv4Addr::new(192, 168, 10, 1).into(),
                    dhcp_enabled: true,
                    auto_ip_enabled: false,
                }),
            },
            Interface {
                typ: "USB".to_string(),
                name: "usb0".to_string(),
                instr_addr_string: vec!["USB::0x1234::125::A22-5::INSTR".to_string()],
                network_info: None,
            },
        ]
    }

    fn get_extended_functions(&self) -> Vec<ExtendedFunction> {
        vec![
            ExtendedFunction::new("LXI VXI-11".to_string(), "1.0".to_string()),
            ExtendedFunction::new("LXI HiSLIP".to_string(), "2.0".to_string()),
        ]
    }
}

pub trait LxiState {
    fn get_device_info(&self) -> DeviceInfo;

    fn get_lxi_info(&self) -> LxiInfo;

    fn set_hostname(&self, hostname: &str);

    fn get_interfaces(&self) -> Vec<Interface>;

    fn get_extended_functions(&self) -> Vec<ExtendedFunction>;
}

pub struct DeviceInfo {
    model: String,
    manufacturer: String,
    serial_number: String,
    fw_version: String,
    description: String,
    homepage_url: String,
    driver_url: String,
    user_description: String,
    identification_url: String,
}

impl DeviceInfo {
    pub fn new() -> Self {
        Self {
            model: "T800 Model 101".to_string(),
            manufacturer: "Cyberdyne systems".to_string(),
            serial_number: "A9012.C".to_string(),
            fw_version: "V2.4".to_string(),
            description: "Sample Device".to_string(),
            homepage_url: "example.com".to_string(),
            driver_url: "example.com".to_string(),
            user_description: "User description".to_string(),
            identification_url: "sampledevice.local/lxi/identification".to_string(),
        }
    }

    pub fn short_descripton(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.manufacturer, self.model, self.serial_number, self.description
        )
    }

    pub fn to_xml<S>(&self, writer: &mut EventWriter<S>) -> Result<(), EmitterError>
    where
        S: io::Write,
    {
        //<Manufacturer>...</Manufacturer>
        writer.write(XmlEvent::start_element("Manufacturer"))?;
        writer.write(self.manufacturer.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<Model>...</Model>
        writer.write(XmlEvent::start_element("Model"))?;
        writer.write(self.model.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<SerialNumber>...</SerialNumber>
        writer.write(XmlEvent::start_element("SerialNumber"))?;
        writer.write(self.serial_number.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<FirmwareRevision>...</FirmwareRevision>
        writer.write(XmlEvent::start_element("FirmwareRevision"))?;
        writer.write(self.fw_version.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<ManufacturerDescription>...</ManufacturerDescription>
        writer.write(XmlEvent::start_element("ManufacturerDescription"))?;
        writer.write(self.description.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<HomepageURL>...</HomepageURL>
        writer.write(XmlEvent::start_element("HomepageURL"))?;
        writer.write(self.homepage_url.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<DriverURL>...</DriverURL>
        writer.write(XmlEvent::start_element("DriverURL"))?;
        writer.write(self.driver_url.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<UserDescription>...</UserDescription>
        writer.write(XmlEvent::start_element("UserDescription"))?;
        writer.write(self.user_description.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<IdentificationURL>...</IdentificationURL>
        writer.write(XmlEvent::start_element("IdentificationURL"))?;
        writer.write(self.identification_url.as_str())?;
        writer.write(XmlEvent::end_element())?;

        Ok(())
    }
}

pub struct LxiInfo {
    ivi_software_module_name: String,
    domain: String,
    lxi_version: String,
}

impl LxiInfo {
    pub fn new(ivi_software_module_name: String, domain: String, lxi_version: String) -> Self {
        Self {
            ivi_software_module_name,
            domain,
            lxi_version,
        }
    }

    pub fn to_xml<S>(&self, writer: &mut EventWriter<S>) -> Result<(), EmitterError>
    where
        S: io::Write,
    {
        //<IVISoftwareModuleName>...</IVISoftwareModuleName>
        writer.write(XmlEvent::start_element("IVISoftwareModuleName"))?;
        writer.write(self.ivi_software_module_name.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<Domain>...</Domain>
        writer.write(XmlEvent::start_element("Domain"))?;
        writer.write(self.domain.as_str())?;
        writer.write(XmlEvent::end_element())?;

        //<LXIVersion>...</LXIVersion>
        writer.write(XmlEvent::start_element("LXIVersion"))?;
        writer.write(self.lxi_version.as_str())?;
        writer.write(XmlEvent::end_element())?;

        Ok(())
    }
}

pub struct ConnectedDevice {
    device_uri: String,
}

impl ConnectedDevice {
    pub fn new(device_uri: String) -> Self {
        Self {
            device_uri: "http://sampledevice.local/devices/device0/".to_string(),
        }
    }

    pub fn to_xml<S>(&self, writer: &mut EventWriter<S>) -> Result<(), EmitterError>
    where
        S: io::Write,
    {
        //<DeviceURI>...</DeviceURI>
        writer.write(XmlEvent::start_element("DeviceURI"))?;
        writer.write(self.device_uri.as_str())?;
        writer.write(XmlEvent::end_element())?;

        Ok(())
    }
}

pub struct ExtendedFunction {
    name: String,
    version: String,
}

impl ExtendedFunction {
    pub fn new(name: String, version: String) -> Self {
        Self { name, version }
    }

    pub fn to_xml<S>(&self, writer: &mut EventWriter<S>) -> Result<(), EmitterError>
    where
        S: io::Write,
    {
        let start_function = XmlEvent::start_element("Function")
            .attr("FunctionName", self.name.as_str())
            .attr("Version", self.version.as_str());
        writer.write(start_function)?;
        writer.write(XmlEvent::end_element())?;

        Ok(())
    }
}

pub struct Interface {
    typ: String,
    name: String,
    instr_addr_string: Vec<String>,
    network_info: Option<NetworkInformation>,
}

pub struct NetworkInformation {
    hostname: String,
    ip_addr: IpAddr,
    subnet_mask: String,
    mac_address: String,
    gateway: IpAddr,
    dhcp_enabled: bool,
    auto_ip_enabled: bool,
}

impl Interface {
    pub fn to_xml<S>(&self, writer: &mut EventWriter<S>) -> Result<(), EmitterError>
    where
        S: io::Write,
    {
        let start_interface = XmlEvent::start_element("Interface")
            .attr("xsi:type", "NetworkInformation")
            .attr("InterfaceType", self.typ.as_str())
            .attr("InterfaceName", self.name.as_str());
        let start_interface = match &self.network_info {
            Some(NetworkInformation {
                ip_addr: IpAddr::V4(..),
                ..
            }) => start_interface.attr("IPType", "IPv4"),
            Some(NetworkInformation {
                ip_addr: IpAddr::V6(..),
                ..
            }) => start_interface.attr("IPType", "IPv6"),
            None => start_interface,
        };
        writer.write(start_interface)?;

        for addr in &self.instr_addr_string {
            //<InstrumentAddressString>...</InstrumentAddressString>
            writer.write(XmlEvent::start_element("InstrumentAddressString"))?;
            writer.write(addr.as_str())?;
            writer.write(XmlEvent::end_element())?;
        }

        if let Some(info) = &self.network_info {
            //<Hostname>...</Hostname>
            writer.write(XmlEvent::start_element("Hostname"))?;
            writer.write(info.hostname.as_str())?;
            writer.write(XmlEvent::end_element())?;

            //<IPAddress>...</IPAddress>
            writer.write(XmlEvent::start_element("IPAddress"))?;
            writer.write(format!("{}", info.ip_addr).as_str())?;
            writer.write(XmlEvent::end_element())?;

            //<SubnetMask>...</SubnetMask>
            writer.write(XmlEvent::start_element("SubnetMask"))?;
            writer.write(info.subnet_mask.as_str())?;
            writer.write(XmlEvent::end_element())?;

            //<MACAddress>...</MACAddress>
            writer.write(XmlEvent::start_element("MACAddress"))?;
            writer.write(info.mac_address.as_str())?;
            writer.write(XmlEvent::end_element())?;

            //<Gateway>...</Gateway>
            writer.write(XmlEvent::start_element("Gateway"))?;
            writer.write(format!("{}", info.gateway).as_str())?;
            writer.write(XmlEvent::end_element())?;

            //<DHCPEnabled>...</DHCPEnabled>
            writer.write(XmlEvent::start_element("DHCPEnabled"))?;
            writer.write(format!("{}", info.dhcp_enabled).as_str())?;
            writer.write(XmlEvent::end_element())?;

            //<AutoIPEnabled>...</AutoIPEnabled>
            writer.write(XmlEvent::start_element("AutoIPEnabled"))?;
            writer.write(format!("{}", info.auto_ip_enabled).as_str())?;
            writer.write(XmlEvent::end_element())?;
        }

        writer.write(XmlEvent::end_element())?;

        Ok(())
    }
}
