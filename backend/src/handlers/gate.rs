use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::{ScanLog, ScanResultResponse, ValidateTicketRequest},
    middleware::auth::RequireGateStaff,
    services::gate_service::GateService,
    AppState,
};

pub async fn validate_ticket(
    State(state): State<AppState>,
    RequireGateStaff(claims): RequireGateStaff,
    Json(req): Json<ValidateTicketRequest>,
) -> Result<Json<ScanResultResponse>, (StatusCode, String)> {
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
    RequireGateStaff(_claims): RequireGateStaff,
    Path(event_id): Path<Uuid>,
) -> Result<Json<Vec<ScanLog>>, (StatusCode, String)> {
    let logs = GateService::get_scan_history(&state.pool, event_id).await?;
    Ok(Json(logs))
}
