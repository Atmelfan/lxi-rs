use serde::{Serialize, Deserialize};

pub const SCHEMA: &'static str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/LXICertificateRequest.xsd"));

/// See LXI-API Extended function 23.16.1
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LxiCertificateRequest")]
struct LxiCertificateRequest {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation")]
    pub xsi_schema_location: String,
    
    #[serde(rename = "SubjectName", skip_serializing_if = "Option::is_none")]
    subject_name: Option<SubjectName>,
    #[serde(rename = "AltDnsName", skip_serializing_if = "Option::is_none")]
    alt_dns_names: Option<Vec<AltDnsName>>,
    #[serde(rename = "AltIpAddress", skip_serializing_if = "Option::is_none")]
    alt_ip_address: Option<Vec<AltIpAddress>>,
    #[serde(rename = "ExpirationDateTime", skip_serializing_if = "Option::is_none")]
    expiration_date_time: Option<String>,
    #[serde(rename = "SignatureAlgorithm", skip_serializing_if = "Option::is_none")]
    signature_algorithm: Option<String>,
    #[serde(rename = "CertificateExtension", skip_serializing_if = "Option::is_none")]
    certificate_extensions: Option<Vec<CertificateExtension>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AltDnsName(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct AltIpAddress(String);

/// See LXI-API Extended function 23.13.2
#[derive(Debug, Serialize, Deserialize)]
struct SubjectName {
    #[serde(rename = "CommonName")]
    common_name: Option<String>,
    #[serde(rename = "Organization")]
    organization: Option<String>,
    #[serde(rename = "OrganizationalUnit")]
    organizational_units: Option<Vec<OrganizationalUnit>>,
    #[serde(rename = "Locality")]
    locality: Option<String>,
    #[serde(rename = "State")]
    state: Option<String>,
    #[serde(rename = "Country")]
    country: Option<String>,
    #[serde(rename = "SerialNumber")]
    serial_number: Option<String>,
    #[serde(rename = "ExtraSubjectAttribute")]
    extra_subject_attributes: Option<Vec<ExtraSubjectAttribute>>,
    
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationalUnit(String);

/// See LXI-API Extended function 23.13.3
#[derive(Debug, Serialize, Deserialize)]
struct ExtraSubjectAttribute {
    #[serde(rename = "ObjectID")]
    object_id: String,
    #[serde(rename = "ObjectValue")]
    object_value: String,
}

/// See LXI-API Extended function 23.13.4
#[derive(Debug, Serialize, Deserialize)]
struct CertificateExtension {
    #[serde(rename = "ObjectID")]
    object_id: String,
    #[serde(rename = "Critical", skip_serializing_if = "Option::is_none")]
    critical: Option<bool>,
    #[serde(rename = "ObjectValue")]
    object_value: String,
}



