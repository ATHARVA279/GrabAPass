use axum::{Router, routing::get};
use crate::AppState;
use crate::handlers::ticket;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(ticket::list_tickets))
        .route("/{id}", get(ticket::get_ticket))
}
