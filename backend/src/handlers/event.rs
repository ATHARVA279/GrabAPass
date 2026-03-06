use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{CreateEventRequest, Event},
    middleware::auth::RequireOrganizer,
    services::event_service,
};

#[derive(Deserialize)]
pub struct EventFilterParams {
    pub category: Option<String>,
    pub search: Option<String>,
}

pub async fn list_published_events(
    State(state): State<AppState>,
    Query(params): Query<EventFilterParams>,
) -> Result<Json<Vec<Event>>, (StatusCode, String)> {
    let events = event_service::list_published_events(
        &state,
        params.category.as_deref(),
        params.search.as_deref().unwrap_or(""),
    )
    .await?;

    Ok(Json(events))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, (StatusCode, String)> {
    let event = event_service::get_event(&state, id).await?;
    Ok(Json(event))
}

pub async fn create_event(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Json(payload): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<Event>), (StatusCode, String)> {
    let event = event_service::create_event(&state, claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn get_organizer_events(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
) -> Result<Json<Vec<Event>>, (StatusCode, String)> {
    let events = event_service::list_organizer_events(&state, claims.sub).await?;
    Ok(Json(events))
}
