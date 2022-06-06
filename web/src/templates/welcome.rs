use askama::Template;

use crate::{ExtendedFunction, Interface};

use super::filters;

#[derive(Template)]
#[template(path = "welcome.html")]
pub struct WelcomeTemplate<'a> {
    model: &'a str,
    manufacturer: &'a str,
    serial_number: &'a str,
    fw_version: &'a str,
    description: &'a str,
    extended_functions: Vec<ExtendedFunction>,
    lxi_version: &'a str,
    interfaces: Vec<Interface>,
}

impl<'a> WelcomeTemplate<'a> {
    pub fn new(
        model: &'a str,
        manufacturer: &'a str,
        serial_number: &'a str,
        fw_version: &'a str,
        description: &'a str,
        extended_functions: Vec<ExtendedFunction>,
        lxi_version: &'a str,
        interfaces: Vec<Interface>,
    ) -> Self {
        Self {
            model,
            manufacturer,
            serial_number,
            fw_version,
            description,
            extended_functions,
            lxi_version,
            interfaces,
        }
    }
}