use serde::{Deserialize, Serialize};

/// See LXI-API Extended function 23.19.1
#[derive(Debug, Serialize, Deserialize)]
pub struct LXIProblemDetails {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub xsi_schema_location: String,

    #[serde(rename = "$unflatten=URL")]
    pub url: String,
    #[serde(rename = "$unflatten=UserActionRequired")]
    pub user_action_required: bool,
    #[serde(rename = "$unflatten=EstimatedTimeToComplete")]
    pub estimated_time_to_complete: Option<u32>,
    #[serde(rename = "$unflatten=Details")]
    pub details: Option<String>,
}