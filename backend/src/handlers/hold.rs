use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::{HoldSeatsRequest, SeatHold},
    middleware::auth::RequireAuth,
    services::hold_service::HoldService,
};

pub async fn hold_seats(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<HoldSeatsRequest>,
) -> Result<(StatusCode, Json<Vec<SeatHold>>), (StatusCode, String)> {
    let holds = HoldService::hold_seats(&state.pool, event_id, claims.sub, payload).await?;
    
    Ok((StatusCode::CREATED, Json(holds)))
}
