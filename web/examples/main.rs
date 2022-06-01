use async_sqlx_session::SqliteSessionStore;
use lxi_web::{routes, websockets::Session, State};
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

    app.at("/").get(Redirect::new("/welcome"));

    app.at("/welcome").get(routes::welcome);

    //
    app.at("/as_middleware")
        .with(WebSocket::new(LxiWebSocketServer::handle))
        .get(|_| async move { Ok("this was not a websocket request") });

    // Serve manual
    app.at("/manual").get(Redirect::new("/manual/index.html"));
    app.at("/manual/").serve_dir("examples/manual/book")?;

    app.listen("127.0.0.1:8000").await?;
    Ok(())
}
