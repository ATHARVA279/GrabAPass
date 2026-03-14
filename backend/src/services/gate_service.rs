use sqlx::PgPool;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::db::models::{ScanLog, TicketDetail};
use crate::repositories::gate_repository::GateRepository;
use crate::repositories::event_repository;

pub struct GateService;

impl GateService {
    pub async fn validate_ticket(
        pool: &PgPool,
        qr_payload: &str,
        event_id: Uuid,
        staff_id: Uuid,
        jwt_secret: &str,
    ) -> Result<(bool, String, Option<TicketDetail>), (StatusCode, String)> {
        if !GateRepository::is_staff_assigned_to_event(pool, event_id, staff_id).await? {
            return Err((StatusCode::FORBIDDEN, "You are not assigned to this event.".to_string()));
        }
        GateRepository::validate_and_admit(pool, qr_payload, event_id, staff_id, jwt_secret).await
    }

    pub async fn get_scan_history(
        pool: &PgPool,
        event_id: Uuid,
        staff_id: Uuid,
    ) -> Result<Vec<ScanLog>, (StatusCode, String)> {
        if !GateRepository::is_staff_assigned_to_event(pool, event_id, staff_id).await? {
            return Err((StatusCode::FORBIDDEN, "You are not assigned to this event.".to_string()));
        }
        GateRepository::list_scan_logs(pool, event_id).await
    }

    pub async fn list_assigned_events(
        pool: &PgPool,
        gate_staff_id: Uuid,
    ) -> Result<Vec<crate::db::models::Event>, (StatusCode, String)> {
        event_repository::list_assigned_events_for_gate_staff(pool, gate_staff_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}
