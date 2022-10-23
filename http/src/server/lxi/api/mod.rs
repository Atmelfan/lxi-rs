pub mod auth;

mod common_configuration {
    use tide::{Request, Response};

    pub use crate::common::lxi::identification::*;

    use super::Permission;

    pub async fn get<S>(req: Request<S>) -> tide::Result {
        let response: tide::Response = "howdy stranger".to_string().into();
        Ok(response)
    }

    pub async fn put<S>(req: Request<S>) -> tide::Result {
        if let Some(_perms) = req.ext::<Permission>() {
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
pub struct Permission;


pub trait UserCredentials {
    fn get_username(&self);
    fn set_password(&mut self);
} 
