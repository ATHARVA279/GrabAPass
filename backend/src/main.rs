pub mod db;
pub mod handlers;
pub mod middleware;
pub mod repositories;
pub mod routes;
pub mod services;

use axum::Router;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub jwt_secret: String,
    pub razorpay: Option<RazorpayConfig>,
    pub rate_limiter: SharedRateLimiter,
}

#[derive(Clone)]
pub struct RazorpayConfig {
    pub key_id: String,
    pub key_secret: String,
    pub webhook_secret: Option<String>,
    pub checkout_name: String,
    pub client: reqwest::Client,
}

pub type SharedRateLimiter = Arc<Mutex<HashMap<String, VecDeque<std::time::Instant>>>>;

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();
    tracing::info!("Starting up the backend server");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let razorpay = match (
        env::var("RAZORPAY_KEY_ID").ok(),
        env::var("RAZORPAY_KEY_SECRET").ok(),
    ) {
        (Some(key_id), Some(key_secret)) => Some(RazorpayConfig {
            key_id,
            key_secret,
            webhook_secret: env::var("RAZORPAY_WEBHOOK_SECRET").ok(),
            checkout_name: env::var("RAZORPAY_CHECKOUT_NAME")
                .unwrap_or_else(|_| "GrabAPass".to_string()),
            client: reqwest::Client::new(),
        }),
        _ => {
            tracing::warn!("Razorpay is not configured. Payment initialization endpoints will be unavailable.");
            None
        }
    };

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

    let state = AppState {
        pool,
        jwt_secret,
        razorpay,
        rate_limiter: Arc::new(Mutex::new(HashMap::new())),
    };

    // Spawn background task to clean up expired holds every 10 seconds
    let bg_pool = state.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = crate::services::hold_service::HoldService::release_expired_holds(&bg_pool).await {
                tracing::error!("Failed to release expired holds in background task: {:?}", e);
            }
        }
    });

    // Set up CORS — restrict to frontend origin
    let allowed_origin =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:4200".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origin
                .parse::<HeaderValue>()
                .expect("Invalid FRONTEND_URL"),
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let app = Router::new()
        .merge(routes::health::router())
        .nest("/api/auth", routes::auth::router())
        .nest("/api/events", routes::event::public_router())
        .nest("/api/orders", routes::order::router())
        .nest("/api/payments", routes::payment::router())
        .nest("/api/tickets", routes::ticket::router())
        .nest("/api/gate", routes::gate::router())
        .nest("/api/organizer/events", routes::event::organizer_router())
        .nest("/api/organizer/venues", routes::venue::organizer_venue_router())
        .layer(cors)
        .with_state(state);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
