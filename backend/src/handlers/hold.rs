use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::{HoldSeatsRequest, SeatHold},
    middleware::auth::RequireCustomer,
    services::hold_service::HoldService,
    services::rate_limit_service::RateLimitService,
};

pub async fn hold_seats(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    RequireCustomer(claims): RequireCustomer,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<HoldSeatsRequest>,
) -> Result<(StatusCode, Json<Vec<SeatHold>>), (StatusCode, String)> {
    let actor = format!("{}:{}", claims.sub, RateLimitService::actor_from_headers(&headers));
    RateLimitService::check_limit(
        &state.rate_limiter,
        "seat_hold",
        &actor,
        12,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let holds = HoldService::hold_seats(&state.pool, event_id, claims.sub, payload).await?;
    
    Ok((StatusCode::CREATED, Json(holds)))
}
