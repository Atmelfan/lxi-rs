use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/LXIPendingDetails.xsd"));

/// See LXI-API Extended function 23.19.1
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXIPendingDetails")]
pub struct LxiPendingDetails {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "URL")]
    pub url: String,
    #[serde(rename = "UserActionRequired")]
    pub user_action_required: bool,
    #[serde(rename = "EstimatedTimeToComplete")]
    pub estimated_time_to_complete: Option<u32>,
    #[serde(rename = "Details")]
    pub details: Option<String>,
}