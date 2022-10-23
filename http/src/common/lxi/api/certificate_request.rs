use serde::{Serialize, Deserialize};

/// See LXI-API Extended function 23.16.1
#[derive(Debug, Serialize, Deserialize)]
struct LXICertificateRequest {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,
    
    #[serde(rename = "$unflatten=SubjectName")]
    subject_name: Option<SubjectName>,
    #[serde(rename = "AltDnsName")]
    alt_dns_names: Vec<AltDnsName>,
    #[serde(rename = "AltIpAddress")]
    alt_ip_address: Vec<AltIpAddress>,
    #[serde(rename = "$unflatten=ExpirationDateTime")]
    expiration_date_time: Option<String>,
    #[serde(rename = "$unflatten=SignatureAlgorithm")]
    signature_algorithm: Option<String>,
    #[serde(rename = "CertificateExtension")]
    certificate_extensions: Vec<CertificateExtension>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AltDnsName(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct AltIpAddress(String);

/// See LXI-API Extended function 23.13.2
#[derive(Debug, Serialize, Deserialize)]
struct SubjectName {
    #[serde(rename = "$unflatten=CommonName")]
    common_name: Option<String>,
    #[serde(rename = "$unflatten=Organization")]
    organization: Option<String>,
    #[serde(rename = "OrganizationalUnit")]
    organizational_units: Vec<OrganizationalUnit>,
    #[serde(rename = "$unflatten=Locality")]
    locality: Option<String>,
    #[serde(rename = "$unflatten=State")]
    state: Option<String>,
    #[serde(rename = "$unflatten=Country")]
    country: Option<String>,
    #[serde(rename = "$unflatten=SerialNumber")]
    serial_number: Option<String>,
    #[serde(rename = "ExtraSubjectAttribute")]
    extra_subject_attributes: Vec<ExtraSubjectAttribute>,
    
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationalUnit(String);

/// See LXI-API Extended function 23.13.3
#[derive(Debug, Serialize, Deserialize)]
struct ExtraSubjectAttribute {
    #[serde(rename = "$unflatten=ObjectID")]
    object_id: String,
    #[serde(rename = "$unflatten=ObjectValue")]
    object_value: String,
}

/// See LXI-API Extended function 23.13.4
#[derive(Debug, Serialize, Deserialize)]
struct CertificateExtension {
    #[serde(rename = "$unflatten=ObjectID")]
    object_id: String,
    #[serde(rename = "$unflatten=Critical")]
    critical: Option<bool>,
    #[serde(rename = "$unflatten=ObjectValue")]
    object_value: String,
}



