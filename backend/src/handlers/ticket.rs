use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    db::models::TicketDetail,
    middleware::auth::RequireAuth,
    services::ticket_service::TicketService,
};

pub async fn list_tickets(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
) -> Result<(StatusCode, Json<Vec<TicketDetail>>), (StatusCode, String)> {
    let tickets = TicketService::list_user_tickets(&state.pool, claims.sub).await?;
    Ok((StatusCode::OK, Json(tickets)))
}

pub async fn get_ticket(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
    Path(ticket_id): Path<Uuid>,
) -> Result<(StatusCode, Json<TicketDetail>), (StatusCode, String)> {
    let ticket = TicketService::get_ticket(&state.pool, claims.sub, ticket_id).await?;
    Ok((StatusCode::OK, Json(ticket)))
}

pub async fn cancel_ticket(
    State(state): State<crate::AppState>,
    RequireAuth(claims): RequireAuth,
    Path(ticket_id): Path<Uuid>,
) -> Result<(StatusCode, Json<TicketDetail>), (StatusCode, String)> {
    let ticket = TicketService::cancel_ticket(&state.pool, claims.sub, ticket_id).await?;
    Ok((StatusCode::OK, Json(ticket)))
}
