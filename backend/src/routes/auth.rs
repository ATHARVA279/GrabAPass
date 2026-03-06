use axum::{Router, routing::post};

use crate::{AppState, handlers::auth};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
}
