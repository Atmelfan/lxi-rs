use serde::{Deserialize, Serialize};
use sqlx::{database::HasArguments, query::{Query, QueryAs}, sqlite::SqliteArguments};

#[derive(sqlx::Type, Debug, Clone)]
#[sqlx(rename = "role", rename_all = "lowercase")]
pub enum UserRole {
    Operator,
    Admin,
    Calibration,
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct User<'a> {
    /// 
    pub id: i64,
    /// Username
    pub name: String,
    /// User is an administrator
    pub role: UserRole,
    /// Hashed password
    /// Empty password requires the user to set a new password
    password: Option<Vec<u8>>,
    /// Salt used to hash password
    salt: Vec<u8>,
    /// Date user was created
    created: i64,
    /// Date user was modified
    updated: i64,
}

impl crate::utils::AsRoute for User<'_> {
    fn as_route(&self) -> std::borrow::Cow<str> {
        format!("/user/{}", self.id).into()
    }
}

impl User {
    pub fn all() -> QueryAs<'static, sqlx::Sqlite, Self, SqliteArguments<'static>> {
        sqlx::query_as("SELECT * FROM users")
    }

    pub fn find_by_id(id: i64) -> QueryAs<'static, sqlx::Sqlite, Self, SqliteArguments<'static>> {
        sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id)
    }

    pub fn delete_by_id(id: i64) -> Query<'static, sqlx::Sqlite, SqliteArguments<'static>> {
        sqlx::query("DELETE FROM users WHERE id = ?").bind(id)
    }

    // pub fn update(&self, partial: PartialArticle) -> Query {
    //     partial.update_by_id(self.id)
    // }

    pub fn verify(&self, unknown: AuthenticateUser) -> bool {

    }
}

/// Admin create user
pub struct CreateUser {
    /// Username
    pub name: String,
    /// User role
    pub role: UserRole,
    /// Password
    /// None forces user to create a password upon first signin
    pub password: Option<String>,
}

impl CreateUser {
    pub fn create(&self) -> Query<'_, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'_>> {
        sqlx::query(
            "INSERT INTO users (name, role, password, salt, created, updated) VALUES (
            $1, $2, DATETIME('now'), DATETIME('now')
          )",
        )
        .bind(&self.name)
        .bind(&self.role)
        .bind(&self.password)
        .bind(&self.salt)
    }
}

/// Admin delete user
pub struct DeleteUser {
    /// Username
    pub name: String,
}

/// Admin update user
pub struct UpdateUser {
    /// Username
    pub name: String,
    /// New password
    pub new_password: Option<String>,
    /// New role
    pub new_role: Option<UserRole>,
}

/// Admin/User update own password
pub struct UpdatePasswordUser {
    /// Username
    pub name: String,
    /// Old password
    pub old_password: String,
    /// New password
    pub new_password: String,
}

/// User login
pub struct AuthenticateUser {
    /// Username
    pub name: String,
    /// Password
    pub password: String,
}

pub enum UserError {
    /// Invalid user id/username
    NoSuchUser,
    /// Password is not correct
    InvalidCredentials,
    /// User has no password and needs to create one
    CreateCredentials,
}

