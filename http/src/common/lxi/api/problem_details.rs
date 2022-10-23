use serde::{Deserialize, Serialize};

/// See LXI-API Extended function 23.18.1
#[derive(Debug, Serialize, Deserialize)]
pub struct LXIProblemDetails {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "$unflatten=Title")]
    pub title: String,
    #[serde(rename = "$unflatten=Detail")]
    pub detail: Option<String>,
    #[serde(rename = "$unflatten=Instance")]
    pub instance: Option<String>,
}