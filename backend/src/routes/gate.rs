use axum::{routing::{get, post}, Router};
use crate::handlers::gate::{validate_ticket, get_scan_history};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", get(crate::handlers::gate::list_assigned_events))
        .route("/validate", post(validate_ticket))
        .route("/events/{id}/scans", get(get_scan_history))
}
