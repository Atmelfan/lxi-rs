use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/LXIProblemDetails.xsd"));

/// See LXI-API Extended function 23.18.1
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXIProblemDetails")]
pub struct LxiProblemDetails {
    /// Scheme information
    #[serde(rename = "@xmlns", skip_deserializing)]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi", skip_deserializing)]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation", skip_deserializing)]
    pub xsi_schema_location: String,

    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Detail", skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(rename = "Instance", skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl LxiProblemDetails {
    pub fn new(
        xmlns: String,
        xmlns_xsi: String,
        xsi_schema_location: String,
        title: String,
        detail: Option<String>,
        instance: Option<String>,
    ) -> Self {
        Self {
            xmlns,
            xmlns_xsi,
            xsi_schema_location,
            title,
            detail,
            instance,
        }
    }

    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        quick_xml::se::to_string(self)
    }
}
