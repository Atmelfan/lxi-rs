use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/LXIDeviceSpecificConfiguration.xsd"
));

/// See LXI-API Extended function 23.13.1
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXIDeviceSpecificConfiguration")]
pub struct LxiDeviceSpecificConfiguration {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "@name", skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "Ipv4Device", skip_serializing_if = "Option::is_none")]
    ipv4_device: Option<Ipv4Device>,
    #[serde(rename = "Ipv6Device", skip_serializing_if = "Option::is_none")]
    ipv6_device: Option<Ipv6Device>,
}

impl LxiDeviceSpecificConfiguration {
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        quick_xml::se::to_string(self)
    }

    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::de::DeError> {
        quick_xml::de::from_str(xml)
    }
}

/// See LXI-API Extended function 23.13.2
#[derive(Debug, Serialize, Deserialize)]
pub struct Ipv4Device {
    #[serde(rename = "@address", skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    #[serde(rename = "@subnetMask", skip_serializing_if = "Option::is_none")]
    subnet_mask: Option<String>,
    #[serde(rename = "@gateway", skip_serializing_if = "Option::is_none")]
    gateway: Option<String>,
    #[serde(rename = "@dns1", skip_serializing_if = "Option::is_none")]
    dns1: Option<String>,
    #[serde(rename = "@dns2", skip_serializing_if = "Option::is_none")]
    dns2: Option<String>,
}

/// See LXI-API Extended function 23.13.3
#[derive(Debug, Serialize, Deserialize)]
pub struct Ipv6Device {
    #[serde(rename = "StaticAddress", skip_serializing_if = "Option::is_none")]
    static_addresses: Option<Vec<IPv6Address>>,
    #[serde(rename = "LinkLocalAddress", skip_serializing_if = "Option::is_none")]
    link_local_address: Option<IPv6Address>,
    #[serde(rename = "GlobalAddress", skip_serializing_if = "Option::is_none")]
    global_addresses: Option<Vec<IPv6Address>>,
}

/// See LXI-API Extended function 23.13.4
#[derive(Debug, Serialize, Deserialize)]
struct IPv6Address {
    #[serde(rename = "@address")]
    address: String,
    #[serde(rename = "@router", skip_serializing_if = "Option::is_none")]
    router: Option<String>,
    #[serde(rename = "@dns", skip_serializing_if = "Option::is_none")]
    dns: Option<String>,
}
