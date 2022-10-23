use serde::{Serialize, Deserialize};

/// See LXI-API Extended function 23.15.1
#[derive(Debug, Serialize, Deserialize)]
struct LXICertificateList {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,
    
    #[serde(rename = "CertificateInfo")]
    certificates: Vec<CertificateInfo>,
}

/// See LXI-API Extended function 23.15.2
#[derive(Debug, Serialize, Deserialize)]
struct CertificateInfo {
    #[serde(rename = "GUID")]
    guid: String,
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "DNSName")]
    dns_name: String,
    #[serde(rename = "Enabled")]
    enabled: bool,
    #[serde(rename = "expirationDateTime")]
    expiration_date_time: String,
}

