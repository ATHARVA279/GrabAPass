use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::models::SuspiciousActivityEvent;

pub struct SuspiciousActivityRepository;

impl SuspiciousActivityRepository {
    pub async fn record_event(
        pool: &PgPool,
        event_id: Uuid,
        user_id: Option<Uuid>,
        ticket_id: Option<Uuid>,
        activity_type: &str,
        severity: &str,
        message: &str,
        metadata: serde_json::Value,
    ) -> Result<(), (StatusCode, String)> {
        sqlx::query(
            r#"
            INSERT INTO suspicious_activity_events (
                event_id,
                user_id,
                ticket_id,
                activity_type,
                severity,
                message,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb)
            "#,
        )
        .bind(event_id)
        .bind(user_id)
        .bind(ticket_id)
        .bind(activity_type)
        .bind(severity)
        .bind(message)
        .bind(metadata)
        .execute(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn count_recent_payment_failures(
        pool: &PgPool,
        event_id: Uuid,
        user_id: Uuid,
        minutes: i64,
    ) -> Result<i64, (StatusCode, String)> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM orders
            WHERE event_id = $1
              AND user_id = $2
              AND status = 'Failed'
              AND created_at >= NOW() - ($3::text || ' minutes')::interval
            "#,
        )
        .bind(event_id)
        .bind(user_id)
        .bind(minutes.to_string())
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn count_recent_rejected_scans(
        pool: &PgPool,
        event_id: Uuid,
        ticket_id: Option<Uuid>,
        minutes: i64,
    ) -> Result<i64, (StatusCode, String)> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM scan_logs
            WHERE event_id = $1
              AND result = 'Rejected'
              AND ($2::uuid IS NULL OR ticket_id = $2)
              AND scanned_at >= NOW() - ($3::text || ' minutes')::interval
            "#,
        )
        .bind(event_id)
        .bind(ticket_id)
        .bind(minutes.to_string())
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn count_recent_duplicate_activity(
        pool: &PgPool,
        event_id: Uuid,
        user_id: Option<Uuid>,
        ticket_id: Option<Uuid>,
        activity_type: &str,
        minutes: i64,
    ) -> Result<i64, (StatusCode, String)> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM suspicious_activity_events
            WHERE event_id = $1
              AND activity_type = $2
              AND ($3::uuid IS NULL OR user_id = $3)
              AND ($4::uuid IS NULL OR ticket_id = $4)
              AND created_at >= NOW() - ($5::text || ' minutes')::interval
            "#,
        )
        .bind(event_id)
        .bind(activity_type)
        .bind(user_id)
        .bind(ticket_id)
        .bind(minutes.to_string())
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn list_recent_for_organizer(
        pool: &PgPool,
        organizer_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SuspiciousActivityEvent>, (StatusCode, String)> {
        sqlx::query_as::<_, SuspiciousActivityEvent>(
            r#"
            SELECT
                sae.id,
                sae.event_id,
                sae.user_id,
                sae.ticket_id,
                sae.activity_type,
                sae.severity,
                sae.message,
                sae.metadata,
                sae.created_at
            FROM suspicious_activity_events sae
            JOIN events e ON e.id = sae.event_id
            WHERE e.organizer_id = $1
            ORDER BY sae.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(organizer_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn count_recent_for_organizer(
        pool: &PgPool,
        organizer_id: Uuid,
        hours: i64,
    ) -> Result<i64, (StatusCode, String)> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM suspicious_activity_events sae
            JOIN events e ON e.id = sae.event_id
            WHERE e.organizer_id = $1
              AND sae.created_at >= NOW() - ($2::text || ' hours')::interval
            "#,
        )
        .bind(organizer_id)
        .bind(hours.to_string())
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}
