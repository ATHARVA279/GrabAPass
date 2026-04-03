use crate::{AppState, handlers::order};
use axum::{Router, routing::get};

pub fn router() -> Router<AppState> {
    Router::new()
        // GET /api/orders — Requires Customer auth
        .route("/", get(order::list_orders))
}
