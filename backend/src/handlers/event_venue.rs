use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{EventVenue, EventVenueInput, EventVenueMatchResponse},
    middleware::auth::RequireOrganizer,
    services::event_venue_service,
};

pub async fn create_or_update_event_venue(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Json(payload): Json<EventVenueInput>,
) -> Result<(StatusCode, Json<EventVenue>), (StatusCode, String)> {
    let venue = event_venue_service::save_event_venue(&state, claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(venue)))
}

pub async fn get_event_venue(
    State(state): State<AppState>,
    RequireOrganizer(_claims): RequireOrganizer,
    Path(id): Path<Uuid>,
) -> Result<Json<EventVenue>, (StatusCode, String)> {
    let venue = event_venue_service::get_event_venue(&state, id).await?;
    Ok(Json(venue))
}

pub async fn match_event_venue(
    State(state): State<AppState>,
    RequireOrganizer(_claims): RequireOrganizer,
    Json(payload): Json<EventVenueInput>,
) -> Result<Json<EventVenueMatchResponse>, (StatusCode, String)> {
    let matches = event_venue_service::find_event_venue_matches(&state, payload).await?;
    Ok(Json(matches))
}
