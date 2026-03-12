use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::{CheckoutRequest, Order},
    middleware::auth::RequireAuth,
    services::order_service::OrderService,
};

pub async fn checkout(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<CheckoutRequest>,
) -> Result<(StatusCode, Json<Order>), (StatusCode, String)> {
    let order = OrderService::checkout(&state.pool, event_id, claims.sub, payload, &state.jwt_secret).await?;
    
    Ok((StatusCode::CREATED, Json(order)))
}

pub async fn list_orders(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
) -> Result<(StatusCode, Json<Vec<Order>>), (StatusCode, String)> {
    let orders = OrderService::get_user_orders(&state.pool, claims.sub).await?;
    
    Ok((StatusCode::OK, Json(orders)))
}
