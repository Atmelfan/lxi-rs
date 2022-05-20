use sqlx::SqlitePool;


pub mod utils;
pub mod templates;
pub mod routes;
pub mod records;

pub type Request = tide::Request<State>;



#[derive(Clone)]
pub struct State {
    pub db: SqlitePool,
}