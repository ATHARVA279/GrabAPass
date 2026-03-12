use sqlx::PgPool;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::db::models::{ScanLog, TicketDetail};
use crate::repositories::gate_repository::GateRepository;

pub struct GateService;

impl GateService {
    pub async fn validate_ticket(
        pool: &PgPool,
        qr_payload: &str,
        event_id: Uuid,
        staff_id: Uuid,
        jwt_secret: &str,
    ) -> Result<(bool, String, Option<TicketDetail>), (StatusCode, String)> {
        GateRepository::validate_and_admit(pool, qr_payload, event_id, staff_id, jwt_secret).await
    }

    pub async fn get_scan_history(
        pool: &PgPool,
        event_id: Uuid,
    ) -> Result<Vec<ScanLog>, (StatusCode, String)> {
        GateRepository::list_scan_logs(pool, event_id).await
    }
}
