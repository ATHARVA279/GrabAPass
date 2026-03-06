use axum::{Router, routing::get};

use crate::{AppState, handlers::health};

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health::health_check))
}
