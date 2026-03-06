use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{CreateEventRequest, Event},
    repositories::event_repository,
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
    event_repository::create_event(
        &state.pool,
        organizer_id,
        &payload.title,
        payload.description.as_deref(),
        &payload.category,
        &payload.venue_name,
        &payload.venue_address,
        payload.start_time,
    )
    .await
    .map_err(internal_error)
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
