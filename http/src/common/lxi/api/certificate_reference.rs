use serde::{Serialize, Deserialize};

/// See LXI-API Extended function 23.14.1
#[derive(Debug, Serialize, Deserialize)]
struct LXICertificateRef {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,
    
    #[serde(rename = "GUID")]
    guid: String,
}

