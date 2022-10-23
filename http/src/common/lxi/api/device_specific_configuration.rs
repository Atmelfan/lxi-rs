use serde::{Deserialize, Serialize};

/// See LXI-API Extended function 23.13.1
#[derive(Debug, Serialize, Deserialize)]
pub struct LxiCommonConfiguration {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,
    
    #[serde(rename = "name")]
    name: Option<String>,
    #[serde(rename = "$unflatten=Ipv4Device")]
    ipv4_device: Option<Ipv4Device>,
    #[serde(rename = "$unflatten=Ipv6Device")]
    ipv6_device: Option<Ipv6Device>,
}

impl LxiCommonConfiguration {
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        let mut buffer = Vec::new();
        let mut writer = quick_xml::Writer::new(&mut buffer);

        // Declaration
        let decl = quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None);
        writer.write_event(quick_xml::events::Event::Decl(decl))?;

        #[derive(Serialize)]
        struct Doc<'a> {
            #[serde(rename = "xmlns")]
            pub xmlns: String,
            #[serde(rename = "xmlns:xsi")]
            pub xmlns_xsi: String,
            #[serde(rename = "xsi:schemaLocation")]
            pub xsi_schema_location: String,
            #[serde(flatten)]
            t: &'a LxiCommonConfiguration,
        }

        let mut ser = quick_xml::se::Serializer::with_root(writer, Some("LXIDevice"));
        Doc {
            xmlns: "http://lxistandard.org/schemas/LXICommonConfiguration/1.0".to_string(),
            xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
            xsi_schema_location:
                "http://lxistandard.org/schemas/LXICommonConfiguration/1.0/LXICommonConfiguration.xsd"
                    .to_string(),
            t: self,
        }.serialize(&mut ser)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

/// See LXI-API Extended function 23.13.2
#[derive(Debug, Serialize, Deserialize)]
pub struct Ipv4Device {
    #[serde(rename = "address")]
    address: Option<String>,
    #[serde(rename = "subnetMask")]
    subnet_mask: Option<String>,
    #[serde(rename = "gateway")]
    gateway: Option<String>,
    #[serde(rename = "dns1")]
    dns1: Option<String>,
    #[serde(rename = "dns2")]
    dns2: Option<String>,
}

/// See LXI-API Extended function 23.13.3
#[derive(Debug, Serialize, Deserialize)]
pub struct Ipv6Device {
    #[serde(rename = "StaticAddress")]
    static_addresses: Vec<IPv6Address>,
    #[serde(rename = "LinkLocalAddress")]
    link_local_address: Option<IPv6Address>,
    #[serde(rename = "GlobalAddress")]
    global_addresses: Vec<IPv6Address>,
}

/// See LXI-API Extended function 23.13.4
#[derive(Debug, Serialize, Deserialize)]
struct IPv6Address {
    #[serde(rename = "address")]
    address: String,
    #[serde(rename = "router")]
    router: Option<String>,
    #[serde(rename = "dns")]
    dns: Option<String>,
}
