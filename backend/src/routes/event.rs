use axum::{Router, routing::{get, post, put}};

use crate::{AppState, handlers::{event, venue}};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(event::list_published_events))
        .route("/{id}", get(event::get_event))
        // GET /api/events/:id/seat-layout — public, used by booking page
        .route("/{id}/seat-layout", get(venue::get_seat_layout))
        // POST /api/events/:id/holds — Requires Customer auth (or any logged-in user)
        .route("/{id}/holds", post(crate::handlers::hold::hold_seats))
        // POST /api/events/:id/checkout/initialize — Creates a pending order and Razorpay order
        .route("/{id}/checkout/initialize", post(crate::handlers::order::initialize_checkout))
        // POST /api/events/:id/checkout/verify — Verifies Razorpay payment and fulfills tickets
        .route("/{id}/checkout/verify", post(crate::handlers::order::verify_checkout))
        // POST /api/events/:id/checkout/failure — Stores failed checkout details
        .route("/{id}/checkout/failure", post(crate::handlers::order::record_checkout_failure))
}

pub fn organizer_router() -> Router<AppState> {
    Router::new()
        .route("/dashboard/summary", get(event::get_organizer_dashboard_summary))
        .route("/", get(event::get_organizer_events).post(event::create_event))
        .route("/{id}", put(event::update_event).delete(event::delete_event))
        // POST /api/organizer/events/:id/seat-categories
        .route("/{id}/seat-categories", post(venue::assign_seat_categories))
}
