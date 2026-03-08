use axum::{Router, routing::get};

use crate::{AppState, handlers::venue};

/// Organizer-only routes: /api/organizer/venues/…
pub fn organizer_venue_router() -> Router<AppState> {
    Router::new()
        .route("/", get(venue::list_venues).post(venue::create_venue))
        .route("/{id}", get(venue::get_venue))
}
