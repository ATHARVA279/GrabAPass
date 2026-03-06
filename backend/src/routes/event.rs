use axum::{Router, routing::get};

use crate::{AppState, handlers::event};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(event::list_published_events))
        .route("/{id}", get(event::get_event))
}

pub fn organizer_router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(event::get_organizer_events).post(event::create_event),
    )
}
