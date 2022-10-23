use serde::{Deserialize, Serialize};

/// See LXI-API Extended function 23.12.1
#[derive(Debug, Serialize, Deserialize)]
pub struct LxiCommonConfiguration {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,
    
    #[serde(rename = "strict")]
    strict: Option<bool>,
    #[serde(rename = "HSMPresent")]
    hsm_present: bool,
    #[serde(rename = "Interface")]
    interfaces: Vec<Interface>,
    #[serde(rename = "$unflatten=ClientAuthentication")]
    client_authenticaton: Option<ClientAuthentication>,
}

/// See LXI-API Extended function 23.12.2
#[derive(Debug, Serialize, Deserialize)]
pub struct Interface {
    #[serde(rename = "name")]
    name: Option<String>,
    #[serde(rename = "LXIConformant")]
    lxi_conformant: Option<String>,
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "unsecureMode")]
    unsecure_mode: Option<bool>,
    #[serde(rename = "otherUnsecureProtocolsEnabled")]
    other_unsecure_protocols_enabled: Option<bool>,
    #[serde(rename = "$unflatten=Network")]
    network: Option<Network>,
    #[serde(rename = "$unflatten=HTTP")]
    http: Option<Http>,
    #[serde(rename = "$unflatten=HTTPS")]
    https: Option<Https>,
    #[serde(rename = "$unflatten=SCPIRaw")]
    scpi_raw: Option<ScpiRaw>,
    #[serde(rename = "$unflatten=Telnet")]
    telnet: Option<Telnet>,
    #[serde(rename = "$unflatten=SCPITLS")]
    scpi_tls: Option<ScpiTls>,
    #[serde(rename = "$unflatten=HiSLIP")]
    hislip: Option<Hislip>,
    #[serde(rename = "$unflatten=VXI11")]
    vxi11: Option<Vxi11>,
}

/// See LXI-API Extended function 23.12.3
#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    #[serde(rename = "$unflatten=IPv4")]
    ipv4: Option<NetworkIpv4>,
    #[serde(rename = "$unflatten=IPv6")]
    ipv6: Option<NetworkIpv6>,
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
    #[serde(rename = "$unflatten=pingEnabled")]
    ping_enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.5
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkIpv6 {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "$unflatten=DHCPEnabled")]
    dhcp_enabled: Option<bool>,
    #[serde(rename = "$unflatten=RAEnabled")]
    ra_enabled: Option<bool>,
    #[serde(rename = "$unflatten=staticAddressEnabled")]
    static_address_enabled: Option<bool>,
    #[serde(rename = "$unflatten=privacyModeEnabled")]
    privacy_mode_enabled: Option<bool>,
    #[serde(rename = "$unflatten=mDNSEnabled")]
    mdns_enabled: Option<bool>,
    #[serde(rename = "$unflatten=dynamicDNSEnabled")]
    dynamic_dns_enabled: Option<bool>,
    #[serde(rename = "$unflatten=pingEnabled")]
    ping_enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.6
#[derive(Debug, Serialize, Deserialize)]
pub struct Http {
    #[serde(rename = "enabled")]
    operation: Option<String>,
    #[serde(rename = "port")]
    port: Option<u16>,
    #[serde(rename = "Service")]
    services: Vec<Service>,
}

/// See LXI-API Extended function 23.12.7
#[derive(Debug, Serialize, Deserialize)]
pub struct Https {
    #[serde(rename = "port")]
    port: Option<u16>,
    #[serde(rename = "clientAuthenticationRequired")]
    client_authentication_required: Option<bool>,
    #[serde(rename = "Service")]
    services: Vec<Service>,
}

/// See LXI-API Extended function 23.12.8
#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    #[serde(rename = "name")]
    name: Option<String>,
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "$unflatten=Basic")]
    basic: Option<AuthenticationMechanism>,
    #[serde(rename = "$unflatten=Digest")]
    digest: Option<AuthenticationMechanism>,
}

/// See LXI-API Extended function 23.12.9
#[derive(Debug, Serialize, Deserialize)]
pub struct ScpiRaw {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "port")]
    port: Option<u16>,
    #[serde(rename = "capability")]
    capability: Option<usize>,
}

/// See LXI-API Extended function 23.12.10
#[derive(Debug, Serialize, Deserialize)]
pub struct ScpiTls {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "port")]
    port: Option<u16>,
    #[serde(rename = "clientAuthenticationRequired")]
    client_authentication_required: Option<bool>,
    #[serde(rename = "capability")]
    capability: Option<usize>,
}

/// See LXI-API Extended function 23.12.11
#[derive(Debug, Serialize, Deserialize)]
pub struct Telnet {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "port")]
    port: Option<u16>,
    #[serde(rename = "TLSRequired")]
    tls_required: Option<bool>,
    #[serde(rename = "clientAuthenticationRequired")]
    client_authentication_required: Option<bool>,
    #[serde(rename = "capability")]
    capability: Option<usize>,
}

/// See LXI-API Extended function 23.12.12
#[derive(Debug, Serialize, Deserialize)]
pub struct Hislip {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
    #[serde(rename = "port")]
    port: Option<u16>,
    #[serde(rename = "mustStartEncrypted")]
    must_start_encrypted: Option<bool>,
    #[serde(rename = "encryptionMandatory")]
    encryption_mandatory: Option<bool>,
}

/// See LXI-API Extended function 23.12.13
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientAuthenticationMechanisms {
    #[serde(rename = "$unflatten=ANONYMOUS")]
    anonymous: Option<AuthenticationMechanism>,
    #[serde(rename = "$unflatten=PLAIN")]
    plain: Option<AuthenticationMechanism>,
    #[serde(rename = "$unflatten=SCRAM")]
    scram: Option<AuthenticationMechanism>,
    #[serde(rename = "$unflatten=MTLS")]
    mtls: Option<AuthenticationMechanism>,
}

/// See LXI-API Extended function 23.12.14
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationMechanism {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.15
#[derive(Debug, Serialize, Deserialize)]
pub struct Vxi11 {
    #[serde(rename = "enabled")]
    enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.16
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientAuthentication {
    #[serde(rename = "scramHashIterationCount")]
    scram_hash_iteration_count: Option<u32>,
    #[serde(rename = "scramChannelBindingRequired")]
    scram_channel_binding_required: Option<bool>,
    #[serde(rename = "$unflatten=ClientCredential")]
    client_credential: Option<ClientCredential>,
    #[serde(rename = "$unflatten=ClientCertAuthentication")]
    client_cert_authentication: Option<ClientCertAuthentication>,
}

/// See LXI-API Extended function 23.12.17
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCredential {
    #[serde(rename = "user")]
    user: String,
    #[serde(rename = "password")]
    password: Option<String>,
    #[serde(rename = "APIAccess")]
    api_access: Option<bool>,
}

/// See LXI-API Extended function 23.12.18
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCertAuthentication {
    #[serde(rename = "RootCertPEM")]
    root_cert_pems: Vec<RootCertPem>,
    #[serde(rename = "CertThumbprint")]
    cert_thumbprints: Vec<CertThumbprint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RootCertPem(String);

/// See LXI-API Extended function 23.12.19
#[derive(Debug, Serialize, Deserialize)]
pub struct CertThumbprint {
    #[serde(rename = "hash")]
    hash: Option<String>,
    #[serde(rename = "thumbPrint")]
    thumb_print: String,
}