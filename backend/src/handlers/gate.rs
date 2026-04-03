use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{ScanLog, ScanResultResponse, ValidateTicketRequest},
    middleware::auth::RequireGateStaff,
    services::gate_service::GateService,
    services::rate_limit_service::RateLimitService,
};

pub async fn validate_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    RequireGateStaff(claims): RequireGateStaff,
    Json(req): Json<ValidateTicketRequest>,
) -> Result<Json<ScanResultResponse>, (StatusCode, String)> {
    let actor = format!(
        "{}:{}",
        claims.sub,
        RateLimitService::actor_from_headers(&headers)
    );
    RateLimitService::check_limit(
        &state.rate_limiter,
        "gate_validate",
        &actor,
        30,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let (success, message, ticket_detail) = GateService::validate_ticket(
        &state.pool,
        &req.qr_payload,
        req.event_id,
        claims.sub,
        &state.jwt_secret,
    )
    .await?;

    Ok(Json(ScanResultResponse {
        success,
        message,
        ticket_detail,
    }))
}

pub async fn get_scan_history(
    State(state): State<AppState>,
    RequireGateStaff(claims): RequireGateStaff,
    Path(event_id): Path<Uuid>,
) -> Result<Json<Vec<ScanLog>>, (StatusCode, String)> {
    let logs = GateService::get_scan_history(&state.pool, event_id, claims.sub).await?;
    Ok(Json(logs))
}

pub async fn list_assigned_events(
    State(state): State<AppState>,
    RequireGateStaff(claims): RequireGateStaff,
) -> Result<Json<Vec<crate::db::models::Event>>, (StatusCode, String)> {
    let events = GateService::list_assigned_events(&state.pool, claims.sub).await?;
    Ok(Json(events))
}
