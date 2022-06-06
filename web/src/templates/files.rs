use askama::Template;

#[derive(Template)]
#[template(path = "files.html")]
pub struct ListDirectory<'a> {
    model: &'a str,
    manufacturer: &'a str,
    serial_number: &'a str,
    description: &'a str,

    error_title: &'a str,
    error_description: &'a str,
}

pub struct File<'a> {

    name: &'a str,
    created: u64,
    modified: u64,
}

pub enum FileType {
    UnknownFile,
    Directory,
    Measurement,
    Calibration
}
