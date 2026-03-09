use axum::{Router, routing::get};
use crate::{AppState, handlers::order};

pub fn router() -> Router<AppState> {
    Router::new()
        // GET /api/orders — Requires Customer auth
        .route("/", get(order::list_orders))
}
