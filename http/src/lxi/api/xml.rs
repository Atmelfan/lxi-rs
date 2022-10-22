use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LxiCommonConfiguration {
    #[serde(rename = "strict")]
    strict: bool,
    #[serde(rename = "HSMPresent")]
    hsm_present: bool,
    #[serde(rename = "Interface")]
    interfaces: Vec<Interface>,
    #[serde(rename = "$unflatten=ClientAuthentication")]
    client_authenticaton: (),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Interface {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "LXIConformant")]
    lxi_conformant: String,
    #[serde(rename = "enabled")]
    enabled: bool,
    #[serde(rename = "unsecureMode")]
    unsecure_mode: bool,
    #[serde(rename = "otherUnsecureProtocolsEnabled")]
    other_unsecure_protocols_enabled: bool,

    #[serde(rename = "$unflatten=Network")]
    network: Option<Network>,
    #[serde(rename = "$unflatten=HTTP")]
    http: (),
    #[serde(rename = "$unflatten=HTTPS")]
    https: (),
    #[serde(rename = "$unflatten=SCPIRaw")]
    scpi_raw: (),
    #[serde(rename = "$unflatten=Telnet")]
    telnet: (),
    #[serde(rename = "$unflatten=SCPITLS")]
    scpi_tls: (),
    #[serde(rename = "$unflatten=HiSLIP")]
    hislip: (),
    #[serde(rename = "$unflatten=VXI11")]
    vxi11: (),
}

/// See LXI-API Extended function 23.12.3
#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    #[serde(rename = "$unflatten=IPv4")]
    ipv4: Option<()>,
    #[serde(rename = "$unflatten=IPv6")]
    ipv6: Option<()>,
}

/// See LXI-API Extended function 23.12.4
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkIpv4 {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "$unflatten=AutoIPEnabled")]
    auto_ip_enabled: Option<bool>,
    #[serde(rename = "$unflatten=DHCPEnabled")]
    dhcp_enabled: Option<bool>,
    #[serde(rename = "$unflatten=mDNSEnabled")]
    mdns_enabled: Option<bool>,
    #[serde(rename = "$unflatten=dynamicDNSEnabled")]
    dynamic_dns_enabled: Option<bool>,
}
