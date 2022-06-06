pub mod articles;

/// /lxi/
pub mod lxi;

use tide::Response;

use crate::{templates::{welcome::*, error::ErrorTemplate}, LxiState};

/// /welcome
pub async fn welcome<S>(request: tide::Request<S>) -> tide::Result where S: LxiState {
    let state = request.state();
    let device_info = state.get_device_info();
    let extended_functions = state.get_extended_functions();
    let lxi_info = state.get_lxi_info();
    let interfaces = state.get_interfaces();
    Ok(WelcomeTemplate::new(
        &device_info.model,
        &device_info.manufacturer,
        &device_info.serial_number,
        &device_info.fw_version,
        &device_info.description,
        extended_functions,
        &lxi_info.lxi_version,
        interfaces,
    )
    .into())
}

/// /error
pub async fn error<S>(request: tide::Request<S>) -> tide::Result where S: LxiState {
    let kind = request.param("code").unwrap_or("404");
    let code: u16 = kind.parse().unwrap_or_default();
    let error = tide::http::StatusCode::try_from(code).unwrap_or(tide::http::StatusCode::NotFound);
    let title = format!("{} ({})", error.canonical_reason(), error as u16);

    let state = request.state();
    let device_info = state.get_device_info();

    Ok(ErrorTemplate::new(
        &device_info.model,
        &device_info.manufacturer,
        &device_info.serial_number,
        &device_info.description,
        &title,
        "Something went wrong, sorry. Please use the navigation bar to continue."
    )
    .into())
}
