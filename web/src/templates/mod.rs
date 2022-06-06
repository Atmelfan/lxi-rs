pub mod articles;

pub mod welcome;
pub mod error;
//pub mod files;

mod filters {
    pub fn id(s: &str) -> ::askama::Result<String> {
        Ok(s.trim().to_lowercase().replace(" ", "-"))
    }
}

pub enum UserInfo<'a> {
    /// Not logged in
    Guest,
    /// Logged in user
    User {
        username: &'a str,
        role: &'a str
    }
}

pub struct UserTemplate {
    
}