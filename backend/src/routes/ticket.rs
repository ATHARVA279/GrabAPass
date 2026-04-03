use crate::AppState;
use crate::handlers::ticket;
use axum::{
    Router,
    routing::{get, post},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(ticket::list_tickets))
        .route("/{id}", get(ticket::get_ticket))
        .route("/{id}/cancel", post(ticket::cancel_ticket))
}
