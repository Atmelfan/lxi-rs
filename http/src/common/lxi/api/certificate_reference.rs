use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/LXICertificateRef.xsd"
));

/// See LXI-API Extended function 23.14.1
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXICertificateRef")]
struct LxiCertificateRef {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "GUID")]
    guid: String,
}
