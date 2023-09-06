use axum::middleware::from_fn;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr};

mod components;
mod config;
mod controllers;
mod db_ops;
mod errors;
mod htmx;
mod middleware;
mod models;
mod routes;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db = create_pg_pool().await;
    let state = models::AppState { db };
    let app = routes::get_routes()
        .layer(from_fn(middleware::html_headers))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn create_pg_pool() -> sqlx::Pool<sqlx::Postgres> {
    let pg_usr = &env::var("POSTGRES_USER")
        .expect("postgres user to be defined in environment")[..];
    let pg_pw = &env::var("POSTGRES_PASSWORD")
        .expect("postgres password to be defined in environment")[..];
    let pg_db = &env::var("POSTGRES_DB")
        .expect("postgres db name to be defined in environment")[..];
    let db_url = &format!(
        "postgres://{}:{}@localhost:5432/{}",
        pg_usr, pg_pw, pg_db
    )[..];

    PgPoolOptions::new()
        // Postgres default max connections is 100, and we'll take 'em
        // https://www.postgresql.org/docs/current/runtime-config-connection.html
        .max_connections(80)
        .connect(db_url)
        .await
        .expect("pool to be able to connect")
}
