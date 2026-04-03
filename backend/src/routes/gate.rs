use crate::AppState;
use crate::handlers::gate::{get_scan_history, validate_ticket};
use axum::{
    Router,
    routing::{get, post},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", get(crate::handlers::gate::list_assigned_events))
        .route("/validate", post(validate_ticket))
        .route("/events/{id}/scans", get(get_scan_history))
}
