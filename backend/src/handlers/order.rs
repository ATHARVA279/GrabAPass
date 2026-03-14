use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::{
        CheckoutFailureRequest, InitializeCheckoutRequest, InitializeCheckoutResponse, Order,
        VerifyCheckoutRequest,
    },
    middleware::auth::RequireAuth,
    services::order_service::OrderService,
    services::rate_limit_service::RateLimitService,
};

pub async fn initialize_checkout(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    RequireAuth(claims): RequireAuth,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<InitializeCheckoutRequest>,
) -> Result<(StatusCode, Json<InitializeCheckoutResponse>), (StatusCode, String)> {
    let actor = format!("{}:{}", claims.sub, RateLimitService::actor_from_headers(&headers));
    RateLimitService::check_limit(
        &state.rate_limiter,
        "checkout_initialize",
        &actor,
        8,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let response = OrderService::initialize_checkout(&state, event_id, &claims, payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn verify_checkout(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    RequireAuth(claims): RequireAuth,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<VerifyCheckoutRequest>,
) -> Result<(StatusCode, Json<Order>), (StatusCode, String)> {
    let actor = format!("{}:{}", claims.sub, RateLimitService::actor_from_headers(&headers));
    RateLimitService::check_limit(
        &state.rate_limiter,
        "checkout_verify",
        &actor,
        12,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let order = OrderService::verify_checkout(&state, event_id, claims.sub, payload).await?;
    Ok((StatusCode::OK, Json(order)))
}

pub async fn record_checkout_failure(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<CheckoutFailureRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    OrderService::record_checkout_failure(&state, event_id, claims.sub, payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_orders(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
) -> Result<(StatusCode, Json<Vec<Order>>), (StatusCode, String)> {
    let orders = OrderService::get_user_orders(&state, claims.sub).await?;
    
    Ok((StatusCode::OK, Json(orders)))
}
