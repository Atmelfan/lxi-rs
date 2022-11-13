use serde::{Deserialize, Serialize};

pub const SCHEMA: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/LXILiterals.xsd"
));

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXILiterals")]
pub struct LxiLiterals<T> {
    /// Scheme information
    #[serde(rename = "@xmlns", skip_deserializing)]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi", skip_deserializing)]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation", skip_deserializing)]
    pub xsi_schema_location: String,

    #[serde(flatten)]
    pub t: T,
}

impl<T> LxiLiterals<T> {
    pub fn to_xml(&self) -> Result<String, quick_xml::DeError>
    where
        T: Serialize,
    {
        quick_xml::se::to_string(self)
    }

    pub fn from_xml<'a>(xml: &'a str) -> Result<Self, quick_xml::de::DeError>
    where
        T: Deserialize<'a>,
    {
        quick_xml::de::from_str(xml)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "LXILiterals")]
pub struct LxiLiteralsBoolean {
    /// Scheme information
    #[serde(rename = "@xmlns", skip_deserializing)]
    pub xmlns: String,
    #[serde(rename = "@xmlns:xsi", skip_deserializing)]
    pub xmlns_xsi: String,
    #[serde(rename = "@xsi:schemaLocation", skip_deserializing)]
    pub xsi_schema_location: String,

    #[serde(rename = "@value")]
    pub value: bool,
}

#[cfg(test)]
mod tests {
    use super::LxiLiterals;

    #[test]
    fn serialize_bool() {
        let x = LxiLiterals {
            xmlns: "".to_string(),
            xmlns_xsi: "".to_string(),
            xsi_schema_location: "".to_string(),
            t: true,
        };
    }
}
