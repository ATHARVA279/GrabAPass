use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::{InitializeSplitRequest, SplitSession, SplitSharePublicDetail, SplitCheckoutInitialization, VerifySplitShareRequest},
    middleware::auth::RequireCustomer,
    repositories::{order_repository::OrderRepository, split_repository::SplitRepository},
    services::split_service::SplitService,
    AppState,
};

pub async fn initialize_split(
    State(state): State<AppState>,
    RequireCustomer(claims): RequireCustomer,
    Path(order_id): Path<Uuid>,
    Json(payload): Json<InitializeSplitRequest>,
) -> Result<(StatusCode, Json<SplitSession>), (StatusCode, String)> {
    let session = SplitService::initialize_split(&state.pool, order_id, claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn get_split_session_for_order(
    State(state): State<AppState>,
    RequireCustomer(claims): RequireCustomer,
    Path(order_id): Path<Uuid>,
) -> Result<(StatusCode, Json<SplitSession>), (StatusCode, String)> {
    let _order = OrderRepository::get_order_by_id_for_user(&state.pool, order_id, claims.sub).await?;
    let session = SplitRepository::get_split_session_for_order_with_shares(&state.pool, order_id).await?;
    Ok((StatusCode::OK, Json(session)))
}

pub async fn get_split_share(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<(StatusCode, Json<SplitSharePublicDetail>), (StatusCode, String)> {
    let detail = SplitRepository::get_split_share_public_details(&state.pool, token).await?;
    Ok((StatusCode::OK, Json(detail)))
}

pub async fn checkout_share(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<(StatusCode, Json<SplitCheckoutInitialization>), (StatusCode, String)> {
    let config = state.razorpay.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Payment gateway is not configured.".to_string(),
    ))?;
    let session = SplitService::initialize_share_checkout(&state.pool, token, config).await?;
    Ok((StatusCode::OK, Json(session)))
}

pub async fn verify_share_payment(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
    Json(payload): Json<VerifySplitShareRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let detail = SplitRepository::get_split_share_public_details(&state.pool, token).await?;

    let gateway_order_id = detail.gateway_order_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Share has no gateway order because checkout has not been initialized yet.".to_string(),
    ))?;

    if payload.razorpay_order_id != gateway_order_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "Payment confirmation did not match the latest split checkout session.".to_string(),
        ));
    }

    SplitService::process_share_payment(
        &state.pool,
        &payload.razorpay_payment_id,
        &gateway_order_id,
        &state.jwt_secret,
    )
    .await?;

    let updated = SplitRepository::get_split_share_public_details(&state.pool, token).await?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "session_status": updated.session_status,
        "order_id": updated.order_id,
    }))))
}

pub async fn claim_share_ticket(
    State(state): State<AppState>,
    RequireCustomer(claims): RequireCustomer,
    Path(token): Path<Uuid>,
) -> Result<(StatusCode, Json<crate::db::models::TicketDetail>), (StatusCode, String)> {
    let ticket = SplitService::claim_share_ticket(
        &state.pool,
        token,
        claims.sub,
        &claims.email,
        &state.jwt_secret,
    )
    .await?;

    Ok((StatusCode::OK, Json(ticket)))
}
