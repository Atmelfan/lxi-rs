use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename = "LXIDevice")]
pub struct Identification {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "$unflatten=Manufacturer")]
    pub manufacturer: String,
    #[serde(rename = "$unflatten=Model")]
    pub model: String,
    #[serde(rename = "$unflatten=SerialNumber")]
    pub serial_number: String,
    #[serde(rename = "$unflatten=FirmwareRevision")]
    pub firmware_revision: String,
    #[serde(rename = "$unflatten=ManufacturerDescription")]
    pub manufacturer_description: String,
    #[serde(rename = "$unflatten=HomepageURL")]
    pub homepage_url: String,
    #[serde(rename = "$unflatten=DriverURL")]
    pub driver_url: String,
    #[serde(rename = "ConnectedDevices", skip_serializing_if = "Option::is_none")]
    pub connected_devices: Option<ConnectedDevices>,
    #[serde(rename = "$unflatten=UserDescription")]
    pub user_description: String,
    #[serde(rename = "$unflatten=IdentificationURL")]
    pub identification_url: String,
    #[serde(rename = "Interface")]
    pub interfaces: Vec<Interface>,
    #[serde(
        rename = "IVISoftwareModuleName",
        skip_serializing_if = "Option::is_none"
    )]
    pub ivisoftware_module_name: Option<IVISoftwareModuleName>,
    #[serde(rename = "$unflatten=Domain", skip_serializing_if = "Option::is_none")]
    pub domain: Option<u8>,
    #[serde(rename = "$unflatten=LXIVersion")]
    pub lxi_version: String,
    #[serde(rename = "LXIExtendedFunctions")]
    pub extended_functions: ExtendedFunctions,
}

impl Identification {
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        let mut buffer = Vec::new();
        let mut writer = quick_xml::Writer::new(&mut buffer);

        // Declaration
        let decl = quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None);
        writer.write_event(quick_xml::events::Event::Decl(decl))?;

        let mut ser = quick_xml::se::Serializer::with_root(writer, Some("LXIDevice"));
        self.serialize(&mut ser)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ConnectedDevices {
    #[serde(rename = "DeviceURI")]
    pub devices: Vec<DeviceUri>,
}


#[derive(Debug, serde::Serialize)]
pub struct IVISoftwareModuleName {
    #[serde(rename = "Comment")]
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
#[serde(untagged)]
pub enum Interface {
    InterfaceInformation {
        #[serde(rename = "InterfaceType")]
        interface_type: String,
        #[serde(rename = "InterfaceName")]
        interface_name: Option<String>,
        #[serde(rename = "InstrumentAddressString")]
        instrument_address_strings: Vec<InstrumentAddressString>,
    },
    NetworkInformation {
        #[serde(rename = "xsi:type")]
        xsi_type: String,
        #[serde(rename = "InterfaceType")]
        interface_type: String,
        #[serde(rename = "IPType")]
        ip_type: IpType,
        #[serde(rename = "InterfaceName")]
        interface_name: Option<String>,
        #[serde(rename = "InstrumentAddressString")]
        instrument_address_strings: Vec<InstrumentAddressString>,
        #[serde(rename = "$unflatten=Hostname")]
        hostname: String,
        #[serde(rename = "$unflatten=IPAddress")]
        ip_address: String,
        #[serde(rename = "$unflatten=SubnetMask")]
        subnet_mask: String,
        #[serde(rename = "$unflatten=MACAddress")]
        mac_address: String,
        #[serde(rename = "$unflatten=Gateway")]
        gateway: String,
        #[serde(rename = "$unflatten=DHCPEnabled")]
        dhcp_enabled: bool,
        #[serde(rename = "$unflatten=AutoIPEnabled")]
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
    #[serde(rename = "$primitive=IPv4")]
    Ipv4,
    #[serde(rename = "$primitive=IPv6")]
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
            xsi_type: "NetworkInformation".to_string(),
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
#[serde(tag = "FunctionName")]
pub enum Function {
    /// [LXI HiSLIP](https://lxistandard.org/members/Adopted%20Specifications/Latest%20Version%20of%20Standards_/LXI%20Version%201.6/LXI_HiSLIP_Extended_Function_1.3_2022-05-26.pdf)
    #[serde(rename = "LXI HiSLIP")]
    Hislip {
        #[serde(rename = "Version")]
        version: String,
        #[serde(rename = "$unflatten=Port")]
        port: u16,
        #[serde(rename = "Subaddress")]
        subaddresses: Vec<Subaddress>,
    },
    #[serde(rename = "LXI VXI-11 Discovery and Identification")]
    Vxi11DiscoveryAndIdentification {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "LXI API")]
    Api {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "LXI IPv6")]
    Ipv6 {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "LXI Event Messaging")]
    EventMessaging {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "LXI Event Log")]
    EventLog {
        #[serde(rename = "Version")]
        version: String,
    },
    #[serde(rename = "LXI Security")]
    Security {
        #[serde(rename = "Version")]
        version: String,
        #[serde(rename = "$unflatten=CryptoSuites")]
        crypto_suites: String,
    },
    
}

#[derive(Debug, Serialize)]
pub struct Subaddress(pub String);