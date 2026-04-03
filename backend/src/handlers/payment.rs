use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::{AppState, services::order_service::OrderService};

pub async fn razorpay_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let signature = headers
        .get("x-razorpay-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing Razorpay signature header.".to_string(),
        ))?;

    let event_id = headers
        .get("x-razorpay-event-id")
        .and_then(|value| value.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Missing Razorpay event id header.".to_string(),
        ))?;

    OrderService::handle_razorpay_webhook(&state, signature, event_id, &body).await?;
    Ok(StatusCode::OK)
}
