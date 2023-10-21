use anyhow::Result;
use axum::{middleware::from_fn, Router};
use dotenvy::dotenv;
use sqlx::{self, postgres::PgPoolOptions};
use std::{env, net::SocketAddr};

mod auth;
mod components;
mod config;
mod controllers;
mod crypto;
mod db_ops;
mod errors;
mod htmx;
mod middleware;
mod models;
mod pw;
mod routes;
mod session;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let db = create_pg_pool().await?;
    sqlx::migrate!().run(&db).await?;
    let state = models::AppState { db };
    let routes = routes::get_protected_routes()
        .layer(from_fn(middleware::html_headers))
        .layer(from_fn(middleware::auth));

    let public_routes =
        routes::get_public_routes().layer(from_fn(middleware::html_headers));

    let app = Router::new()
        .nest("/", routes)
        .nest("/", public_routes)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn create_pg_pool() -> Result<sqlx::Pool<sqlx::Postgres>> {
    let db_url = &env::var("DATABASE_URL")
        .expect("database url to be defined in the environment")[..];

    Ok(PgPoolOptions::new()
        // Postgres default max connections is 100, and we'll take 'em
        // https://www.postgresql.org/docs/current/runtime-config-connection.html
        .max_connections(80)
        .connect(db_url)
        .await?)
}
