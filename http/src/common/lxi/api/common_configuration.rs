use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/LXICommonConfiguration.xsd"
));

/// See LXI-API Extended function 23.12.1
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXICommonConfiguration")]
pub struct LxiCommonConfiguration {
    /// Scheme information
    #[serde(rename = "@xmlns", skip_deserializing)]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi", skip_deserializing)]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation", skip_deserializing)]
    pub xsi_schema_location: String,

    #[serde(rename = "@strict", skip_serializing)]
    pub strict: Option<bool>,
    #[serde(rename = "@HSMPresent")]
    pub hsm_present: Option<bool>,
    #[serde(rename = "Interface")]
    pub interfaces: Vec<Interface>,
    #[serde(
        rename = "ClientAuthentication",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_authenticaton: Option<ClientAuthentication>,
}

impl LxiCommonConfiguration {
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        quick_xml::se::to_string(self)
    }

    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::de::DeError> {
        quick_xml::de::from_str(xml)
    }
}

/// See LXI-API Extended function 23.12.2
#[derive(Debug, Serialize, Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@LXIConformant")]
    pub lxi_conformant: Option<String>,
    #[serde(rename = "@enabled")]
    pub enabled: Option<bool>,
    #[serde(rename = "@unsecureMode")]
    pub unsecure_mode: Option<bool>,
    #[serde(rename = "@otherUnsecureProtocolsEnabled")]
    pub other_unsecure_protocols_enabled: Option<bool>,
    #[serde(rename = "Network", skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    #[serde(rename = "HTTP", skip_serializing_if = "Option::is_none")]
    pub http: Option<Vec<Http>>,
    #[serde(rename = "HTTPS", skip_serializing_if = "Option::is_none")]
    pub https: Option<Vec<Https>>,
    #[serde(rename = "SCPIRaw", skip_serializing_if = "Option::is_none")]
    pub scpi_raw: Option<Vec<ScpiRaw>>,
    #[serde(rename = "Telnet", skip_serializing_if = "Option::is_none")]
    pub telnet: Option<Vec<Telnet>>,
    #[serde(rename = "SCPITLS", skip_serializing_if = "Option::is_none")]
    pub scpi_tls: Option<Vec<ScpiTls>>,
    #[serde(rename = "HiSLIP", skip_serializing_if = "Option::is_none")]
    pub hislip: Option<Hislip>,
    #[serde(rename = "VXI11", skip_serializing_if = "Option::is_none")]
    pub vxi11: Option<Vxi11>,
}

/// See LXI-API Extended function 23.12.3
#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    #[serde(rename = "IPv4", skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<NetworkIpv4>,
    #[serde(rename = "IPv6", skip_serializing_if = "Option::is_none")]
    pub ipv6: Option<NetworkIpv6>,
}

/// See LXI-API Extended function 23.12.4
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkIpv4 {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "@autoIPEnabled", skip_serializing_if = "Option::is_none")]
    pub auto_ip_enabled: Option<bool>,
    #[serde(rename = "@DHCPEnabled", skip_serializing_if = "Option::is_none")]
    pub dhcp_enabled: Option<bool>,
    #[serde(rename = "@mDNSEnabled", skip_serializing_if = "Option::is_none")]
    pub mdns_enabled: Option<bool>,
    #[serde(rename = "@dynamicDNSEnabled", skip_serializing_if = "Option::is_none")]
    pub dynamic_dns_enabled: Option<bool>,
    #[serde(rename = "@pingEnabled", skip_serializing_if = "Option::is_none")]
    pub ping_enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.5
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkIpv6 {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "@DHCPEnabled", skip_serializing_if = "Option::is_none")]
    pub dhcp_enabled: Option<bool>,
    #[serde(rename = "@RAEnabled", skip_serializing_if = "Option::is_none")]
    pub ra_enabled: Option<bool>,
    #[serde(
        rename = "@staticAddressEnabled",
        skip_serializing_if = "Option::is_none"
    )]
    pub static_address_enabled: Option<bool>,
    #[serde(
        rename = "@privacyModeEnabled",
        skip_serializing_if = "Option::is_none"
    )]
    pub privacy_mode_enabled: Option<bool>,
    #[serde(rename = "@mDNSEnabled", skip_serializing_if = "Option::is_none")]
    pub mdns_enabled: Option<bool>,
    #[serde(rename = "@dynamicDNSEnabled", skip_serializing_if = "Option::is_none")]
    pub dynamic_dns_enabled: Option<bool>,
    #[serde(rename = "@pingEnabled", skip_serializing_if = "Option::is_none")]
    pub ping_enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.6
#[derive(Debug, Serialize, Deserialize)]
pub struct Http {
    #[serde(rename = "@operation", skip_serializing_if = "Option::is_none")]
    pub operation: Option<HttpOperation>,
    #[serde(rename = "@port", skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(rename = "Service", skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<Service>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HttpOperation {
    #[serde(rename = "enable")]
    Enable,
    #[serde(rename = "disable")]
    Disable,
    #[serde(rename = "redirectAll")]
    RedirectAll,
}

/// See LXI-API Extended function 23.12.7
#[derive(Debug, Serialize, Deserialize)]
pub struct Https {
    #[serde(rename = "@port", skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(
        rename = "@clientAuthenticationRequired",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_authentication_required: Option<bool>,
    #[serde(rename = "Service", skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<Service>>,
}

/// See LXI-API Extended function 23.12.8
#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@enabled")]
    pub enabled: bool,
    #[serde(rename = "Basic", skip_serializing_if = "Option::is_none")]
    pub basic: Option<AuthenticationMechanism>,
    #[serde(rename = "Digest", skip_serializing_if = "Option::is_none")]
    pub digest: Option<AuthenticationMechanism>,
}

/// See LXI-API Extended function 23.12.9
#[derive(Debug, Serialize, Deserialize)]
pub struct ScpiRaw {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "@port", skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(rename = "@capability", skip_serializing_if = "Option::is_none")]
    pub capability: Option<usize>,
}

/// See LXI-API Extended function 23.12.10
#[derive(Debug, Serialize, Deserialize)]
pub struct ScpiTls {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "@port")]
    pub port: u16,
    #[serde(
        rename = "@clientAuthenticationRequired",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_authentication_required: Option<bool>,
    #[serde(rename = "@capability", skip_serializing_if = "Option::is_none")]
    pub capability: Option<usize>,
}

/// See LXI-API Extended function 23.12.11
#[derive(Debug, Serialize, Deserialize)]
pub struct Telnet {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "@port", skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(rename = "@TLSRequired", skip_serializing_if = "Option::is_none")]
    pub tls_required: Option<bool>,
    #[serde(
        rename = "@clientAuthenticationRequired",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_authentication_required: Option<bool>,
    #[serde(rename = "@capability", skip_serializing_if = "Option::is_none")]
    pub capability: Option<usize>,
}

/// See LXI-API Extended function 23.12.12
#[derive(Debug, Serialize, Deserialize)]
pub struct Hislip {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "@port", skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(
        rename = "@mustStartEncrypted",
        skip_serializing_if = "Option::is_none"
    )]
    pub must_start_encrypted: Option<bool>,
    #[serde(
        rename = "@encryptionMandatory",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_mandatory: Option<bool>,
    #[serde(
        rename = "ClientAuthenticationMechanisms",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_authentication_mechanisms: Option<ClientAuthenticationMechanisms>,
}

/// See LXI-API Extended function 23.12.13
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientAuthenticationMechanisms {
    #[serde(rename = "ANONYMOUS", skip_serializing_if = "Option::is_none")]
    pub anonymous: Option<AuthenticationMechanism>,
    #[serde(rename = "PLAIN", skip_serializing_if = "Option::is_none")]
    pub plain: Option<AuthenticationMechanism>,
    #[serde(rename = "SCRAM", skip_serializing_if = "Option::is_none")]
    pub scram: Option<AuthenticationMechanism>,
    #[serde(rename = "MTLS", skip_serializing_if = "Option::is_none")]
    pub mtls: Option<AuthenticationMechanism>,
}

/// See LXI-API Extended function 23.12.14
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationMechanism {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.15
#[derive(Debug, Serialize, Deserialize)]
pub struct Vxi11 {
    #[serde(rename = "@enabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// See LXI-API Extended function 23.12.16
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientAuthentication {
    #[serde(
        rename = "@scramHashIterationCount",
        skip_serializing_if = "Option::is_none"
    )]
    pub scram_hash_iteration_count: Option<u32>,
    #[serde(
        rename = "@scramChannelBindingRequired",
        skip_serializing_if = "Option::is_none"
    )]
    pub scram_channel_binding_required: Option<bool>,
    #[serde(rename = "ClientCredential", skip_serializing_if = "Option::is_none")]
    pub client_credential: Option<Vec<ClientCredential>>,
    #[serde(
        rename = "ClientCertAuthentication",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_cert_authentication: Option<ClientCertAuthentication>,
}

/// See LXI-API Extended function 23.12.17
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCredential {
    #[serde(rename = "@user")]
    pub user: String,
    #[serde(rename = "@password", skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "@APIAccess", skip_serializing_if = "Option::is_none")]
    pub api_access: Option<bool>,
}

impl ClientCredential {}

/// See LXI-API Extended function 23.12.18
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientCertAuthentication {
    #[serde(rename = "RootCertPEM")]
    pub root_cert_pems: Vec<RootCertPem>,
    #[serde(rename = "CertThumbprint")]
    pub cert_thumbprints: Vec<CertThumbprint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RootCertPem(pub String);

/// See LXI-API Extended function 23.12.19
#[derive(Debug, Serialize, Deserialize)]
pub struct CertThumbprint {
    #[serde(rename = "@hash", skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(rename = "@thumbPrint")]
    pub thumb_print: String,
}
