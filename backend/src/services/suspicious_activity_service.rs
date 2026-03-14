use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    db::models::SuspiciousActivityEvent,
    repositories::suspicious_activity_repository::SuspiciousActivityRepository,
};

pub struct SuspiciousActivityService;

impl SuspiciousActivityService {
    pub async fn record_payment_failure_if_suspicious(
        pool: &sqlx::PgPool,
        event_id: Uuid,
        user_id: Uuid,
        order_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), (StatusCode, String)> {
        let failures =
            SuspiciousActivityRepository::count_recent_payment_failures(pool, event_id, user_id, 15).await?;

        if failures < 3 {
            return Ok(());
        }

        let duplicates = SuspiciousActivityRepository::count_recent_duplicate_activity(
            pool,
            event_id,
            Some(user_id),
            None,
            "repeated_payment_failures",
            30,
        )
        .await?;

        if duplicates > 0 {
            return Ok(());
        }

        SuspiciousActivityRepository::record_event(
            pool,
            event_id,
            Some(user_id),
            None,
            "repeated_payment_failures",
            "high",
            "Multiple payment failures detected for the same customer in a short window.",
            serde_json::json!({
                "order_id": order_id,
                "reason": reason,
                "failures_in_15_minutes": failures,
            }),
        )
        .await
    }

    pub async fn record_rejected_scan_if_suspicious(
        pool: &sqlx::PgPool,
        event_id: Uuid,
        ticket_id: Option<Uuid>,
        reason: &str,
    ) -> Result<(), (StatusCode, String)> {
        let rejected =
            SuspiciousActivityRepository::count_recent_rejected_scans(pool, event_id, ticket_id, 10).await?;

        if rejected < 3 {
            return Ok(());
        }

        let duplicates = SuspiciousActivityRepository::count_recent_duplicate_activity(
            pool,
            event_id,
            None,
            ticket_id,
            "repeated_rejected_scans",
            30,
        )
        .await?;

        if duplicates > 0 {
            return Ok(());
        }

        SuspiciousActivityRepository::record_event(
            pool,
            event_id,
            None,
            ticket_id,
            "repeated_rejected_scans",
            "medium",
            "Repeated rejected scans were detected for this ticket or event.",
            serde_json::json!({
                "ticket_id": ticket_id,
                "reason": reason,
                "rejections_in_10_minutes": rejected,
            }),
        )
        .await
    }

    pub async fn list_recent_for_organizer(
        pool: &sqlx::PgPool,
        organizer_id: Uuid,
    ) -> Result<Vec<SuspiciousActivityEvent>, (StatusCode, String)> {
        SuspiciousActivityRepository::list_recent_for_organizer(pool, organizer_id, 8).await
    }

    pub async fn count_recent_for_organizer(
        pool: &sqlx::PgPool,
        organizer_id: Uuid,
    ) -> Result<i64, (StatusCode, String)> {
        SuspiciousActivityRepository::count_recent_for_organizer(pool, organizer_id, 24).await
    }
}
