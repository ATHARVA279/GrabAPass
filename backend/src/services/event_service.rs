use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{CreateEventRequest, Event},
    repositories::event_repository,
    services::venue_service,
};

pub async fn list_published_events(
    state: &AppState,
    category: Option<&str>,
    search: &str,
) -> Result<Vec<Event>, (StatusCode, String)> {
    event_repository::list_published_events(&state.pool, category, search)
        .await
        .map_err(internal_error)
}

pub async fn get_event(state: &AppState, id: Uuid) -> Result<Event, (StatusCode, String)> {
    event_repository::find_event_by_id(&state.pool, id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))
}

pub async fn create_event(
    state: &AppState,
    organizer_id: Uuid,
    payload: CreateEventRequest,
) -> Result<Event, (StatusCode, String)> {
    let seating_mode =
        venue_service::resolve_seating_mode(payload.seating_mode, payload.venue_template_id.is_some());

    let event = event_repository::create_event(
        &state.pool,
        organizer_id,
        &payload.title,
        payload.description.as_deref(),
        &payload.category,
        &payload.venue_name,
        &payload.venue_address,
        payload.start_time,
        payload.venue_template_id,
        seating_mode,
    )
    .await
    .map_err(internal_error)?;

    // If a venue template was attached, initialise the seat inventory immediately
    if let Some(template_id) = event.venue_template_id {
        venue_service::initialise_event_inventory(state, event.id, template_id).await?;
    }

    Ok(event)
}

pub async fn list_organizer_events(
    state: &AppState,
    organizer_id: Uuid,
) -> Result<Vec<Event>, (StatusCode, String)> {
    event_repository::list_organizer_events(&state.pool, organizer_id)
        .await
        .map_err(internal_error)
}

fn internal_error(error: sqlx::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

