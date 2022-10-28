use tide::{Request, Response};

use crate::common::lxi;

pub async fn identification<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::identification::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn certificate_list<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::certificate_list::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn certificate_reference<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::certificate_reference::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn certificate_request<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::certificate_request::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn common_configuration<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::common_configuration::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn device_specific_configuration<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::device_specific_configuration::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn literals<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::literals::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn pending_details<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::pending_details::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}

pub async fn problem_details<S>(_req: Request<S>) -> tide::Result {
    let mut response: Response = lxi::api::problem_details::SCHEMA.into();
    response.set_content_type("application/xml");
    Ok(response)
}
