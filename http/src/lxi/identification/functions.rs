use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "FunctionName")]
pub enum Function {
    #[serde(rename = "LXI HiSLIP")]
    Hislip {
        #[serde(rename = "Version")]
        version: String,
        #[serde(rename = "$unflatten=Port")]
        port: u16,
        #[serde(rename = "Subaddress")]
        subaddresses: Vec<Subaddress>,
    },
    #[serde(rename = "LXI VXI-11 Discovery and Identification")]
    Vxi11DiscoveryAndIdentification {
        #[serde(rename = "Version")]
        version: String,
    },
}

#[derive(Debug, Serialize)]
pub struct Subaddress(pub String);

#[derive(Debug, Serialize)]
pub struct Hislip {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "$unflatten=Port")]
    pub port: u16,
    #[serde(rename = "Subaddress")]
    pub subaddresses: Vec<Subaddress>,
}
