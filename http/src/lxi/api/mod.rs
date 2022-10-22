pub mod auth;
pub mod xml;

trait LxiApiProvider {
    fn check_api_key();
}
