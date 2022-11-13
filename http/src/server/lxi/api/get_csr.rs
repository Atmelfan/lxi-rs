use tide::Request;

use super::middleware::ProblemDetails;

pub async fn all<S>(_req: Request<S>) -> tide::Result {
    let mut response: tide::Response = tide::http::StatusCode::NotImplemented.into();
    response.insert_ext(ProblemDetails::with_detail(
        "Certificate management not yet implemented",
        None,
    ));
    return Ok(response);
}
