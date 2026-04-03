use axum::{Router, routing::post};

use crate::{AppState, handlers::payment};

pub fn router() -> Router<AppState> {
    Router::new().route("/razorpay/webhook", post(payment::razorpay_webhook))
}
