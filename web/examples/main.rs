use async_sqlx_session::SqliteSessionStore;
use lxi_web::{routes, templates::error::ErrorTemplate, State};
use sqlx::sqlite::SqlitePool;
use std::{collections::BTreeMap, env, sync::Arc, time::Duration};
use tide::{sessions::SessionMiddleware, Redirect};
use tide_websockets::WebSocket;

async fn db_connection() -> tide::Result<SqlitePool> {
    let database_url = env::var("DATABASE_URL")?;
    Ok(SqlitePool::connect(&database_url).await?)
}

async fn build_session_middleware(
    db: SqlitePool,
) -> tide::Result<SessionMiddleware<SqliteSessionStore>> {
    let session_store = SqliteSessionStore::from_client(db);
    session_store.migrate().await?;
    session_store.spawn_cleanup_task(Duration::from_secs(60 * 15));
    let session_secret = env::var("TIDE_SECRET").unwrap();
    Ok(SessionMiddleware::new(
        session_store,
        session_secret.as_bytes(),
    ))
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::with_level(tide::log::LevelFilter::Info);
    let db = db_connection().await?;

    let state = State { db: db.clone() };

    let mut app = tide::with_state(state);
    app.with(build_session_middleware(db).await?);

    // Welcome page
    app.at("/").get(Redirect::new("/welcome"));
    app.at("/welcome").get(routes::welcome);

    // LXI stuff
    app.at("/lxi/identification")
        .get(routes::lxi::identification);

    // Serve assets
    app.at("/assets").serve_dir("examples/assets")?;

    // Handle error page
    app.with(tide::utils::After(|res: tide::Response| async move {
        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            return Ok(Redirect::new(format!("/error/{}", status as u16)).into());
        }
        Ok(res)
    }));
    app.at("/error").get(routes::error);
    app.at("/error/:code").get(routes::error);

    // Serve manual
    let mut manual_app = tide::new();
    manual_app.at("/").serve_dir("examples/user-manual/book")?;
    manual_app.with(tide::utils::After(|res: tide::Response| async move {
        match res.status() {
            tide::http::StatusCode::NotFound => Ok(Redirect::new("/user-manual/404.html").into()),
            _ => Ok(res),
        }
    }));
    app.at("/user-manual").nest(manual_app);
    app.at("/user-manual").get(Redirect::new("/user-manual/index.html"));

    // Start server
    app.listen("127.0.0.1:8000").await?;
    Ok(())
}
