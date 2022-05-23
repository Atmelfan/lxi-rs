use serde::{Deserialize, Serialize};
use sqlx::{database::HasArguments, query::{Query, QueryAs}, sqlite::SqliteArguments};

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub admin: bool,
    created: i32,
    updated: i32,
}

impl crate::utils::AsRoute for User {
    fn as_route(&self) -> std::borrow::Cow<str> {
        format!("/user/{}", self.id).into()
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PartialArticle {
    pub name: Option<String>,
    pub title: Option<String>,
}

impl PartialArticle {
    pub fn update_by_id(&self, id: i64) -> Query<'_, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'_>> {
        sqlx::query(
            "UPDATE articles (text, title, updated) VALUES (
            COALESCE($1, articles.text),
            COALESCE($2, articles.title),
            datetime('now')
          ) WHERE id = $3",
        )
        .bind(&self.text)
        .bind(&self.title)
        .bind(id)
    }

    pub fn create(&self) -> Query<'_, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'_>> {
        sqlx::query(
            "INSERT INTO articles (text, title, created, updated) VALUES (
            $1, $2, DATETIME('now'), DATETIME('now')
          )",
        )
        .bind(&self.text)
        .bind(&self.title)
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
}
