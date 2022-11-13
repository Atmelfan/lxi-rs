use serde::Serialize;

pub const SCHEMA: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/LXIIdentification.xsd"
));

#[derive(Debug, Serialize)]
#[serde(rename = "LXIDevice")]
pub struct LXIIdentification {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "Manufacturer")]
    pub manufacturer: String,
    #[serde(rename = "Model")]
    pub model: String,
    #[serde(rename = "SerialNumber")]
    pub serial_number: String,
    #[serde(rename = "FirmwareRevision")]
    pub firmware_revision: String,
    #[serde(rename = "ManufacturerDescription")]
    pub manufacturer_description: String,
    #[serde(rename = "HomepageURL")]
    pub homepage_url: String,
    #[serde(rename = "DriverURL")]
    pub driver_url: String,
    #[serde(
        rename = "ConnectedDevices",
        skip_serializing_if = "ConnectedDevices::is_empty"
    )]
    pub connected_devices: ConnectedDevices,
    #[serde(rename = "UserDescription")]
    pub user_description: String,
    #[serde(rename = "IdentificationURL")]
    pub identification_url: String,
    #[serde(rename = "Interface")]
    pub interfaces: Vec<Interface>,
    #[serde(rename = "IVISoftwareModuleName")]
    pub ivisoftware_module_name: Vec<IVISoftwareModuleName>,
    #[serde(rename = "Domain", skip_serializing_if = "Option::is_none")]
    pub domain: Option<u8>,
    #[serde(rename = "LXIVersion")]
    pub lxi_version: String,
    #[serde(rename = "LXIExtendedFunctions")]
    pub extended_functions: ExtendedFunctions,
}

impl LXIIdentification {
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        quick_xml::se::to_string(self)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ConnectedDevices {
    #[serde(rename = "DeviceURI")]
    pub devices: Vec<DeviceUri>,
}

impl ConnectedDevices {
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct IVISoftwareModuleName {
    #[serde(rename = "@Comment")]
    pub comment: Option<String>,
    #[serde(rename = "$value")]
    pub name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename = "DeviceURI")]
pub struct DeviceUri {
    #[serde(rename = "$value")]
    pub device_uri: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ExtendedFunctions {
    #[serde(rename = "Function")]
    pub extended_functions: Vec<Function>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "@xsi:type")]
pub enum Interface {
    InterfaceInformation {
        #[serde(rename = "@InterfaceType")]
        interface_type: String,
        #[serde(rename = "@InterfaceName")]
        interface_name: Option<String>,
        #[serde(rename = "InstrumentAddressString")]
        instrument_address_strings: Vec<InstrumentAddressString>,
    },
    NetworkInformation {
        #[serde(rename = "@InterfaceType")]
        interface_type: String,
        #[serde(rename = "@IPType")]
        ip_type: IpType,
        #[serde(rename = "@InterfaceName")]
        interface_name: Option<String>,
        #[serde(rename = "InstrumentAddressString")]
        instrument_address_strings: Vec<InstrumentAddressString>,
        #[serde(rename = "Hostname")]
        hostname: String,
        #[serde(rename = "IPAddress")]
        ip_address: String,
        #[serde(rename = "SubnetMask")]
        subnet_mask: String,
        #[serde(rename = "MACAddress")]
        mac_address: String,
        #[serde(rename = "Gateway")]
        gateway: String,
        #[serde(rename = "DHCPEnabled")]
        dhcp_enabled: bool,
        #[serde(rename = "AutoIPEnabled")]
        auto_ip_enabled: bool,
    },
}

#[derive(Debug, serde::Serialize)]
pub struct InstrumentAddressString {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, serde::Serialize)]
pub enum IpType {
    #[serde(rename = "IPv4")]
    Ipv4,
    #[serde(rename = "IPv6")]
    Ipv6,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_serializer() {
        let value = Interface::InterfaceInformation {
            interface_type: "MyCompanyCustomNetworkInterface".to_string(),
            interface_name: Some("MyCompany1".to_string()),
            instrument_address_strings: vec![InstrumentAddressString {
                value: "10.1.2.32:5025".to_string(),
            }],
        };
        let xml = quick_xml::se::to_string(&value).unwrap();
        assert_eq!(xml, "<Interface InterfaceType=\"MyCompanyCustomNetworkInterface\" InterfaceName=\"MyCompany1\"><InstrumentAddressString>10.1.2.32:5025</InstrumentAddressString></Interface>");

        let value = Interface::NetworkInformation {
            interface_type: "LXI".to_string(),
            interface_name: Some("eth0".to_string()),
            ip_type: IpType::Ipv4,
            instrument_address_strings: vec![
                InstrumentAddressString {
                    value: "TCPIP::10.1.2.32::INSTR".to_string(),
                },
                InstrumentAddressString {
                    value: "TCPIP::10.1.2.32::5000::SOCKET".to_string(),
                },
                InstrumentAddressString {
                    value: "TCPIP::10.1.2.32::hislip0::INSTR".to_string(),
                },
            ],
            hostname: "10.1.2.32".to_string(),
            ip_address: "10.1.2.32".to_string(),
            subnet_mask: "255.255.255.0".to_string(),
            mac_address: "00:3F:F8:6A:1A:3A".to_string(),
            gateway: "10.1.2.1".to_string(),
            dhcp_enabled: true,
            auto_ip_enabled: true,
        };
        let xml = quick_xml::se::to_string(&value).unwrap();
        assert_eq!(xml, "<Interface xsi:type=\"NetworkInformation\" InterfaceType=\"LXI\" IPType=\"IPv4\" InterfaceName=\"eth0\"><InstrumentAddressString>TCPIP::10.1.2.32::INSTR</InstrumentAddressString><InstrumentAddressString>TCPIP::10.1.2.32::5000::SOCKET</InstrumentAddressString><InstrumentAddressString>TCPIP::10.1.2.32::hislip0::INSTR</InstrumentAddressString><Hostname>10.1.2.32</Hostname><IPAddress>10.1.2.32</IPAddress><SubnetMask>255.255.255.0</SubnetMask><MACAddress>00:3F:F8:6A:1A:3A</MACAddress><Gateway>10.1.2.1</Gateway><DHCPEnabled>true</DHCPEnabled><AutoIPEnabled>true</AutoIPEnabled></Interface>")
    }
}

/// LXI extended functions
#[derive(Debug, Serialize)]
#[serde(tag = "@FunctionName")]
pub enum Function {
    /// [LXI HiSLIP](https://lxistandard.org/members/Adopted%20Specifications/Latest%20Version%20of%20Standards_/LXI%20Version%201.6/LXI_HiSLIP_Extended_Function_1.3_2022-05-26.pdf)
    #[serde(rename = "LXI HiSLIP")]
    Hislip {
        #[serde(rename = "@Version")]
        version: String,
        #[serde(rename = "Port")]
        port: u16,
        #[serde(rename = "Subaddress")]
        subaddresses: Vec<Subaddress>,
    },
    #[serde(rename = "LXI VXI-11 Discovery and Identification")]
    Vxi11DiscoveryAndIdentification {
        #[serde(rename = "@Version")]
        version: String,
    },
    #[serde(rename = "LXI API")]
    Api {
        #[serde(rename = "@Version")]
        version: String,
    },
    #[serde(rename = "LXI IPv6")]
    Ipv6 {
        #[serde(rename = "@Version")]
        version: String,
    },
    #[serde(rename = "@LXI Event Messaging")]
    EventMessaging {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "@LXI Event Log")]
    EventLog {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "@LXI Security")]
    Security {
        #[serde(rename = "Version")]
        version: String,
        #[serde(rename = "CryptoSuites")]
        crypto_suites: String,
    },
}

#[derive(Debug, Serialize)]
pub struct Subaddress(pub String);
