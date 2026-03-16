use axum::http::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

use crate::constants::{scan_reason, scan_result, ticket_status};
use crate::db::models::{ScanLog, Ticket, TicketDetail};
use crate::repositories::ticket_repository::TicketRepository;
use crate::services::suspicious_activity_service::SuspiciousActivityService;

pub struct GateRepository;

impl GateRepository {
    pub async fn is_staff_assigned_to_event(
        pool: &PgPool,
        event_id: Uuid,
        staff_id: Uuid,
    ) -> Result<bool, (StatusCode, String)> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM gate_staff_event_assignments
            WHERE event_id = $1 AND gate_staff_id = $2
            "#,
        )
        .bind(event_id)
        .bind(staff_id)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(count > 0)
    }

    /// Validates QR payload and marks ticket as used in a transaction, logging the scan.
    pub async fn validate_and_admit(
        pool: &PgPool,
        qr_payload: &str,
        event_id: Uuid,
        staff_id: Uuid,
        jwt_secret: &str,
    ) -> Result<(bool, String, Option<TicketDetail>), (StatusCode, String)> {
        // Parse payload "{ticket_id}:{qr_secret}"
        let parts: Vec<&str> = qr_payload.split(':').collect();
        if parts.len() != 2 {
            Self::insert_scan_log(pool, None, event_id, staff_id, scan_result::REJECTED, scan_reason::INVALID_QR_FORMAT).await?;
            let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                pool,
                event_id,
                None,
                scan_reason::INVALID_QR_FORMAT,
            )
            .await;
            return Ok((false, "Invalid QR code format".to_string(), None));
        }

        let ticket_id_str = parts[0];
        let provided_secret = parts[1];

        let ticket_id = match Uuid::from_str(ticket_id_str) {
            Ok(id) => id,
            Err(_) => {
                Self::insert_scan_log(pool, None, event_id, staff_id, scan_result::REJECTED, scan_reason::INVALID_TICKET_ID).await?;
                let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                    pool,
                    event_id,
                    None,
                    scan_reason::INVALID_TICKET_ID,
                )
                .await;
                return Ok((false, "Invalid Ticket ID".to_string(), None));
            }
        };

        // Recompute the HMAC and compare
        let expected_secret = TicketRepository::generate_qr_secret(&ticket_id, jwt_secret);
        if provided_secret != expected_secret {
            Self::insert_scan_log(pool, Some(ticket_id), event_id, staff_id, scan_result::REJECTED, scan_reason::QR_SECRET_MISMATCH).await?;
            let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                pool,
                event_id,
                Some(ticket_id),
                scan_reason::QR_SECRET_MISMATCH,
            )
            .await;
            return Ok((false, "Cryptographic signature invalid".to_string(), None));
        }

        // Start transaction for atomic read/update
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        // Fetch ticket with row-level lock
        let ticket: Option<Ticket> = sqlx::query_as::<_, Ticket>(
            r#"
            SELECT id, order_id, event_id, user_id,
                   qr_secret, status, created_at, used_at
            FROM tickets
            WHERE id = $1
            FOR UPDATE
            "#
        )
        .bind(ticket_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let ticket = match ticket {
            Some(t) => t,
            None => {
                let _ = tx.rollback().await;
                Self::insert_scan_log(pool, Some(ticket_id), event_id, staff_id, scan_result::REJECTED, scan_reason::TICKET_NOT_FOUND).await?;
                let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                    pool,
                    event_id,
                    Some(ticket_id),
                    scan_reason::TICKET_NOT_FOUND,
                )
                .await;
                return Ok((false, "Ticket not found".to_string(), None));
            }
        };

        // Validate event match
        if ticket.event_id != event_id {
            let _ = tx.rollback().await;
            Self::insert_scan_log(pool, Some(ticket_id), event_id, staff_id, scan_result::REJECTED, scan_reason::WRONG_EVENT).await?;
            let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                pool,
                event_id,
                Some(ticket_id),
                scan_reason::WRONG_EVENT,
            )
            .await;
            return Ok((false, "Ticket is for a different event".to_string(), None));
        }

        // Validate status
        if ticket.status == ticket_status::USED {
            let ticket_detail = TicketRepository::get_ticket_detail_in_tx(&mut tx, ticket_id).await?;
            let _ = tx.rollback().await;
            Self::insert_scan_log(pool, Some(ticket_id), event_id, staff_id, scan_result::REJECTED, scan_reason::ALREADY_USED).await?;
            let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                pool,
                event_id,
                Some(ticket_id),
                scan_reason::ALREADY_USED,
            )
            .await;
            return Ok((false, "Ticket has already been used".to_string(), Some(ticket_detail)));
        }
        if ticket.status == ticket_status::CANCELLED {
            let ticket_detail = TicketRepository::get_ticket_detail_in_tx(&mut tx, ticket_id).await?;
            let _ = tx.rollback().await;
            Self::insert_scan_log(pool, Some(ticket_id), event_id, staff_id, scan_result::REJECTED, scan_reason::CANCELLED).await?;
            let _ = SuspiciousActivityService::record_rejected_scan_if_suspicious(
                pool,
                event_id,
                Some(ticket_id),
                scan_reason::CANCELLED,
            )
            .await;
            return Ok((false, "Ticket is cancelled".to_string(), Some(ticket_detail)));
        }

        // Admit: Update status and log
        sqlx::query(
            "UPDATE tickets SET status = 'Used', used_at = NOW() WHERE id = $1"
        )
        .bind(ticket_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Self::insert_scan_log_tx(&mut tx, Some(ticket_id), event_id, staff_id, scan_result::ADMITTED, scan_reason::VALID_ENTRY).await?;
        
        // Fetch detailed ticket info to return
        let ticket_detail = TicketRepository::get_ticket_detail_in_tx(&mut tx, ticket_id).await?;

        tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok((true, "Entry Approved".to_string(), Some(ticket_detail)))
    }

    /// Insert a scan log without an active transaction
    pub async fn insert_scan_log(
        pool: &PgPool,
        ticket_id: Option<Uuid>,
        event_id: Uuid,
        scanned_by: Uuid,
        result: &str,
        reason: &str,
    ) -> Result<(), (StatusCode, String)> {
        sqlx::query(
            r#"
            INSERT INTO scan_logs (ticket_id, event_id, scanned_by, result, reason)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(ticket_id)
        .bind(event_id)
        .bind(scanned_by)
        .bind(result)
        .bind(reason)
        .execute(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    /// Insert a scan log within an active transaction
    pub async fn insert_scan_log_tx(
        tx: &mut Transaction<'_, Postgres>,
        ticket_id: Option<Uuid>,
        event_id: Uuid,
        scanned_by: Uuid,
        result: &str,
        reason: &str,
    ) -> Result<(), (StatusCode, String)> {
        sqlx::query(
            r#"
            INSERT INTO scan_logs (ticket_id, event_id, scanned_by, result, reason)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(ticket_id)
        .bind(event_id)
        .bind(scanned_by)
        .bind(result)
        .bind(reason)
        .execute(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    /// List scan history for an event
    pub async fn list_scan_logs(
        pool: &PgPool,
        event_id: Uuid,
    ) -> Result<Vec<ScanLog>, (StatusCode, String)> {
        sqlx::query_as::<_, ScanLog>(
            r#"
            SELECT id, ticket_id, event_id, scanned_by, result, reason, scanned_at
            FROM scan_logs
            WHERE event_id = $1
            ORDER BY scanned_at DESC
            LIMIT 50
            "#
        )
        .bind(event_id)
        .fetch_all(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}
