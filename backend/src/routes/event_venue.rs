use axum::{
    Router,
    routing::{get, post},
};

use crate::{AppState, handlers::event_venue};

pub fn organizer_router() -> Router<AppState> {
    Router::new()
        .route("/", post(event_venue::create_or_update_event_venue))
        .route("/match", post(event_venue::match_event_venue))
        .route("/{id}", get(event_venue::get_event_venue))
}
