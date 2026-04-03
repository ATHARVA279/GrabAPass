use axum::{
    Json,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        AssignGateStaffRequest, CreateEventRequest, Event, EventTicketTier, GateStaffSummary,
        OrganizerDashboardSummaryResponse, PublicEvent,
    },
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
) -> Result<Json<Vec<PublicEvent>>, (StatusCode, String)> {
    tracing::debug!(category = ?params.category, search = ?params.search, "Listing published events");
    let events = event_service::list_published_events(
        &state,
        params.category.as_deref(),
        params.search.as_deref().unwrap_or(""),
    )
    .await?;
    tracing::debug!(count = events.len(), "Returning published events");
    Ok(Json(events))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, (StatusCode, String)> {
    tracing::debug!(event_id = %id, "Fetching event");
    let event = event_service::get_event(&state, id).await?;
    Ok(Json(event))
}

pub async fn get_event_details(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::models::EventDetailsResponse>, (StatusCode, String)> {
    let details = event_service::get_event_details(&state, id).await?;
    Ok(Json(details))
}

pub async fn create_event(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Json(payload): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<Event>), (StatusCode, String)> {
    tracing::info!(organizer_id = %claims.sub, title = %payload.title, "Creating event");
    let event = event_service::create_event(&state, claims.sub, payload).await?;
    tracing::info!(event_id = %event.id, "Event created");
    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn update_event(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<Event>, (StatusCode, String)> {
    let event = event_service::update_event(&state, claims.sub, id, payload).await?;
    Ok(Json(event))
}

pub async fn delete_event(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    event_service::delete_event(&state, claims.sub, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel_event(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, (StatusCode, String)> {
    let event = event_service::cancel_event(&state, claims.sub, id).await?;
    Ok(Json(event))
}

pub async fn get_organizer_events(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
) -> Result<Json<Vec<Event>>, (StatusCode, String)> {
    tracing::debug!(organizer_id = %claims.sub, "Fetching organizer events");
    let events = event_service::list_organizer_events(&state, claims.sub).await?;
    Ok(Json(events))
}

pub async fn get_organizer_event(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, (StatusCode, String)> {
    let event = event_service::get_organizer_event(&state, claims.sub, id).await?;
    Ok(Json(event))
}

pub async fn get_organizer_dashboard_summary(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
) -> Result<Json<OrganizerDashboardSummaryResponse>, (StatusCode, String)> {
    let summary = event_service::get_organizer_dashboard_summary(&state, claims.sub).await?;
    Ok(Json(summary))
}

pub async fn list_gate_staff_users(
    State(state): State<AppState>,
    RequireOrganizer(_claims): RequireOrganizer,
) -> Result<Json<Vec<GateStaffSummary>>, (StatusCode, String)> {
    let users = event_service::list_gate_staff_users(&state).await?;
    Ok(Json(users))
}

pub async fn list_assigned_gate_staff(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<GateStaffSummary>>, (StatusCode, String)> {
    let assigned = event_service::list_assigned_gate_staff(&state, claims.sub, id).await?;
    Ok(Json(assigned))
}

pub async fn assign_gate_staff(
    State(state): State<AppState>,
    RequireOrganizer(claims): RequireOrganizer,
    Path(id): Path<Uuid>,
    Json(payload): Json<AssignGateStaffRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    event_service::assign_gate_staff(&state, claims.sub, id, payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_event_pulse(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::models::EventPulseResponse>, (StatusCode, String)> {
    let pulse = event_service::get_event_pulse(&state, id).await?;
    Ok(Json(pulse))
}

pub async fn get_event_tiers(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<EventTicketTier>>, (StatusCode, String)> {
    let tiers = event_service::get_event_ticket_tiers(&state, id).await?;
    Ok(Json(tiers))
}

pub async fn event_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_event_socket(socket, state, id))
}

async fn handle_event_socket(mut socket: WebSocket, state: AppState, event_id: Uuid) {
    let mut rx = crate::services::ws_service::WsService::get_or_create_channel(&state, event_id)
        .await
        .subscribe();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            res = socket.recv() => {
                match res {
                    Some(Ok(_)) => {
                        // Keep alive: process or ignore incoming messages / ping / pong
                    }
                    _ => break, // Socket disconnected
                }
            }
        }
    }
}
