use axum::{Router, routing::{get, post}};

use crate::{AppState, handlers::{event, venue}};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(event::list_published_events))
        .route("/{id}", get(event::get_event))
        // GET /api/events/:id/seat-layout — public, used by booking page
        .route("/{id}/seat-layout", get(venue::get_seat_layout))
}

pub fn organizer_router() -> Router<AppState> {
    Router::new()
        .route("/", get(event::get_organizer_events).post(event::create_event))
        // POST /api/organizer/events/:id/seat-categories
        .route("/{id}/seat-categories", post(venue::assign_seat_categories))
}

