pub mod auth;

mod common_configuration {
    use tide::{Request, Response};

    pub use crate::common::lxi::identification::*;

    use super::Permissions;

    pub async fn get<S>(req: Request<S>) -> tide::Result {
        let response: tide::Response = "howdy stranger".to_string().into();
        Ok(response)
    }

    pub async fn put<S>(req: Request<S>) -> tide::Result {
        if let Some(_perms) = req.ext::<Permissions>() {
            Ok(format!(
                "API key ok"
            )
            .into())
        } else {
            let mut response: tide::Response = "howdy stranger".to_string().into();
            response.set_status(tide::http::StatusCode::Unauthorized);
            response.insert_header("WWW-Authenticate", "Basic");
            Ok(response)
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Permissions {

}
