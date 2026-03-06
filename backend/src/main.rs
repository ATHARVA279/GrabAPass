pub mod handlers_auth;
pub mod middleware;
pub mod models;

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();
    tracing::info!("Starting up the backend server");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(
            PgConnectOptions::from_str(&database_url)
                .expect("Invalid DATABASE_URL")
                .statement_cache_capacity(0),
        )
        .await
        .expect("Failed to create Postgres connection pool!");

    let state = AppState { pool, jwt_secret };

    // Set up CORS — restrict to frontend origin
    let allowed_origin =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:4200".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origin
                .parse::<HeaderValue>()
                .expect("Invalid FRONTEND_URL"),
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let auth_routes = Router::new()
        .route("/register", post(handlers_auth::register))
        .route("/login", post(handlers_auth::login));

    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes)
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
