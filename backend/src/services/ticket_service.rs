use sqlx::PgPool;
use uuid::Uuid;
use axum::http::StatusCode;
use crate::db::models::TicketDetail;
use crate::repositories::ticket_repository::TicketRepository;

pub struct TicketService;

impl TicketService {
    pub async fn list_user_tickets(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<TicketDetail>, (StatusCode, String)> {
        TicketRepository::list_user_tickets(pool, user_id).await
    }

    pub async fn get_ticket(
        pool: &PgPool,
        user_id: Uuid,
        ticket_id: Uuid,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        TicketRepository::get_ticket_by_id(pool, ticket_id, user_id).await
    }
}
