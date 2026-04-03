use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;

pub fn split_routes() -> Router<AppState> {
    Router::new()
        // Initialize a split checkout for an order
        .route("/api/orders/{id}/split", post(crate::handlers::split::initialize_split))
        // Fetch split session for an order
        .route("/api/orders/{id}/split", get(crate::handlers::split::get_split_session_for_order))
        // Fetch public details of a split share via token
        .route("/api/split/{token}", get(crate::handlers::split::get_split_share))
        // Open Razorpay session for this share
        .route("/api/split/{token}/checkout", post(crate::handlers::split::checkout_share))
        // Verify payment after Razorpay callback (used when webhook is unavailable in local dev)
        .route("/api/split/{token}/verify", post(crate::handlers::split::verify_share_payment))
        // Claim the allocated ticket(s) for this share after the split completes
        .route("/api/split/{token}/claim", post(crate::handlers::split::claim_share_ticket))
}
