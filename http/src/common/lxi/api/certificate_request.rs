use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/LXICertificateRequest.xsd"
));

/// See LXI-API Extended function 23.16.1
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename = "LxiCertificateRequest")]
pub struct LxiCertificateRequest {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "SubjectName", skip_serializing_if = "Option::is_none")]
    pub subject_name: Option<SubjectName>,
    #[serde(rename = "AltDnsName", skip_serializing_if = "Option::is_none")]
    pub alt_dns_names: Option<Vec<AltDnsName>>,
    #[serde(rename = "AltIpAddress", skip_serializing_if = "Option::is_none")]
    pub alt_ip_address: Option<Vec<AltIpAddress>>,
    #[serde(rename = "ExpirationDateTime", skip_serializing_if = "Option::is_none")]
    pub expiration_date_time: Option<String>,
    #[serde(rename = "SignatureAlgorithm", skip_serializing_if = "Option::is_none")]
    pub signature_algorithm: Option<String>,
    #[serde(
        rename = "CertificateExtension",
        skip_serializing_if = "Option::is_none"
    )]
    pub certificate_extensions: Option<Vec<CertificateExtension>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AltDnsName(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct AltIpAddress(pub String);

/// See LXI-API Extended function 23.13.2
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SubjectName {
    #[serde(rename = "CommonName")]
    pub common_name: Option<String>,
    #[serde(rename = "Organization")]
    pub organization: Option<String>,
    #[serde(rename = "OrganizationalUnit")]
    pub organizational_units: Option<Vec<OrganizationalUnit>>,
    #[serde(rename = "Locality")]
    pub locality: Option<String>,
    #[serde(rename = "State")]
    pub state: Option<String>,
    #[serde(rename = "Country")]
    pub country: Option<String>,
    #[serde(rename = "SerialNumber")]
    pub serial_number: Option<String>,
    #[serde(rename = "ExtraSubjectAttribute")]
    pub extra_subject_attributes: Option<Vec<ExtraSubjectAttribute>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationalUnit(String);

/// See LXI-API Extended function 23.13.3
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtraSubjectAttribute {
    #[serde(rename = "ObjectID")]
    pub object_id: String,
    #[serde(rename = "ObjectValue")]
    pub object_value: String,
}

/// See LXI-API Extended function 23.13.4
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateExtension {
    #[serde(rename = "ObjectID")]
    pub object_id: String,
    #[serde(rename = "Critical", skip_serializing_if = "Option::is_none")]
    pub critical: Option<bool>,
    #[serde(rename = "ObjectValue")]
    pub object_value: String,
}
