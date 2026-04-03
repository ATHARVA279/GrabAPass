pub mod constants;
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
use tokio::sync::{Mutex, broadcast};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub jwt_secret: String,
    pub razorpay: Option<RazorpayConfig>,
    pub email: Option<EmailConfig>,
    pub rate_limiter: SharedRateLimiter,
    pub event_channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
}

#[derive(Clone)]
pub struct RazorpayConfig {
    pub key_id: String,
    pub key_secret: String,
    pub webhook_secret: Option<String>,
    pub checkout_name: String,
    pub client: reqwest::Client,
}

#[derive(Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
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
            tracing::warn!(
                "Razorpay is not configured. Payment initialization endpoints will be unavailable."
            );
            None
        }
    };

    let email = match (
        env::var("SMTP_HOST").ok(),
        env::var("SMTP_PORT").ok(),
        env::var("SMTP_USERNAME").ok(),
        env::var("SMTP_PASSWORD").ok(),
        env::var("EMAIL_FROM").ok(),
    ) {
        (
            Some(smtp_host),
            Some(smtp_port),
            Some(smtp_username),
            Some(smtp_password),
            Some(from_email),
        ) => {
            let smtp_port = smtp_port.parse::<u16>().unwrap_or(587);

            tracing::info!(
                host = %smtp_host,
                port = smtp_port,
                from = %from_email,
                "SMTP email notifications are configured."
            );

            Some(EmailConfig {
                smtp_host,
                smtp_port,
                smtp_username,
                smtp_password,
                from_email,
            })
        }
        _ => {
            tracing::warn!(
                "SMTP email notifications are not configured. Booking and refund emails will be skipped."
            );
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

    if std::env::var("RUN_MIGRATIONS").ok().as_deref() == Some("true") {
        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("Failed to run database migrations");
    }

    let state = AppState {
        pool,
        jwt_secret,
        razorpay,
        email,
        rate_limiter: Arc::new(Mutex::new(HashMap::new())),
        event_channels: Arc::new(Mutex::new(HashMap::new())),
    };

    // Spawn background task to clean up expired holds every 10 seconds
    let bg_pool = state.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) =
                crate::services::hold_service::HoldService::release_expired_holds(&bg_pool).await
            {
                tracing::error!(
                    "Failed to release expired holds in background task: {:?}",
                    e
                );
            }
        }
    });

    // Spawn background task to expire split sessions every 10 seconds
    let bg_pool = state.pool.clone();
    let bg_razorpay = state.razorpay.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Some(ref razorpay_config) = bg_razorpay {
                if let Err(e) = crate::services::split_service::SplitService::expire_split_sessions(
                    &bg_pool,
                    razorpay_config,
                )
                .await
                {
                    tracing::error!(
                        "Failed to expire split sessions in background task: {:?}",
                        e
                    );
                }
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
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let app = Router::new()
        .merge(routes::health::router())
        .nest("/api/auth", routes::auth::router())
        .nest("/api/bookings", routes::booking::router())
        .nest("/api/events", routes::event::public_router())
        .nest("/api/orders", routes::order::router())
        .nest("/api/payments", routes::payment::router())
        .nest("/api/tickets", routes::ticket::router())
        .nest("/api/gate", routes::gate::router())
        .nest("/api/organizer/events", routes::event::organizer_router())
        .nest(
            "/api/organizer/event-venues",
            routes::event_venue::organizer_router(),
        )
        .nest(
            "/api/organizer/venues",
            routes::venue::organizer_venue_router(),
        )
        .merge(routes::split::split_routes())
        .layer(cors)
        .with_state(state);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
