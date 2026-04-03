use crate::AppState;
use crate::handlers::ticket;
use axum::{Router, routing::post};

pub fn router() -> Router<AppState> {
    Router::new().route("/{id}/cancel", post(ticket::cancel_ticket))
}
