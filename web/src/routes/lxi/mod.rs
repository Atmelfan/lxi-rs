use tide::http::mime;

use crate::{DeviceInfo, LxiState};

/// /lxi/identification
pub async fn identification<S>(request: tide::Request<S>) -> tide::Result
where
    S: LxiState,
{
    let state = request.state();

    let s = Vec::new();
    let mut writer = xml::EmitterConfig::new()
        .perform_indent(true)
        .create_writer(s);

    // Start LXIDevice
    let start_lxi_device = xml::writer::XmlEvent::start_element("LXIDevice")
        .default_ns("http://www.lxistandard.org/InstrumentIdentification/1.0")
        .ns("xsi", "http://www.w3.org/2001/XMLSchema-instance")
        .attr(
            "xsi:schemaLocation",
            "http://www.lxistandard.org/InstrumentIdentification/1.0 identification.xsd",
        );
    writer.write(start_lxi_device)?;

    // Device information
    state
        .get_device_info()
        .to_xml(&mut writer)
        .map_err(|err| tide::Error::new(500, err))?;

    // Interfaces
    for interface in state.get_interfaces() {
        interface.to_xml(&mut writer)?;
    }

    // Device information
    state
        .get_lxi_info()
        .to_xml(&mut writer)
        .map_err(|err| tide::Error::new(500, err))?;

    // Extended functions
    writer.write(xml::writer::XmlEvent::start_element("LXIExtendedFunctions"))?;
    for function in state.get_extended_functions() {
        function.to_xml(&mut writer)?;
    }
    writer.write(xml::writer::XmlEvent::end_element())?;

    // End LXIDevice
    writer.write(xml::writer::XmlEvent::end_element())?;

    Ok(tide::Response::builder(200)
        .body(writer.into_inner())
        .content_type(mime::XML)
        .build())
}
