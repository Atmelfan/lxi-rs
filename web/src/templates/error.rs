use askama::Template;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate<'a> {
    model: &'a str,
    manufacturer: &'a str,
    serial_number: &'a str,
    description: &'a str,

    error_title: &'a str,
    error_description: &'a str,
}

impl<'a> ErrorTemplate<'a> {
    pub fn new(
        model: &'a str,
        manufacturer: &'a str,
        serial_number: &'a str,
        description: &'a str,
        error_title: &'a str,
        error_description: &'a str,
    ) -> Self {
        Self {
            model,
            manufacturer,
            serial_number,
            description,
            error_title,
            error_description,
        }
    }
}
