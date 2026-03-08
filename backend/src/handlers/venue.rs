use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        AssignSeatCategoryRequest, CreateVenueRequest, SeatLayoutResponse, VenueTemplate,
    },
    middleware::auth::RequireOrganizer,
    services::venue_service,
};

// ─── POST /api/organizer/venues ───────────────────────────────────────────────
pub async fn create_venue(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Json(payload): Json<CreateVenueRequest>,
) -> Result<(StatusCode, Json<VenueTemplate>), (StatusCode, String)> {
    tracing::info!(organizer_id = %claims.sub, venue = %payload.name, "Creating venue template");
    let template = venue_service::create_venue_template(&state, claims.sub, payload).await?;
    tracing::info!(venue_id = %template.id, "Venue template created");
    Ok((StatusCode::CREATED, Json(template)))
}

// ─── GET /api/organizer/venues ────────────────────────────────────────────────
pub async fn list_venues(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
) -> Result<Json<Vec<VenueTemplate>>, (StatusCode, String)> {
    let templates = venue_service::list_venue_templates(&state, claims.sub).await?;
    Ok(Json(templates))
}

// ─── GET /api/organizer/venues/:id ───────────────────────────────────────────
pub async fn get_venue(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
) -> Result<Json<VenueTemplate>, (StatusCode, String)> {
    let template = venue_service::get_venue_template(&state, id, claims.sub).await?;
    Ok(Json(template))
}

// ─── POST /api/organizer/events/:id/seat-categories ──────────────────────────
pub async fn assign_seat_categories(
    State(state): State<AppState>,
    RequireOrganizer(_claims): RequireOrganizer,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<Vec<AssignSeatCategoryRequest>>,
) -> Result<StatusCode, (StatusCode, String)> {
    venue_service::assign_seat_categories(&state, event_id, payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── GET /api/events/:id/seat-layout ─────────────────────────────────────────
// Public endpoint — used by the booking page to render the seat map.
pub async fn get_seat_layout(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<SeatLayoutResponse>, (StatusCode, String)> {
    tracing::debug!(event_id = %event_id, "Fetching seat layout");
    let layout = venue_service::get_seat_layout(&state, event_id).await?;
    Ok(Json(layout))
}
