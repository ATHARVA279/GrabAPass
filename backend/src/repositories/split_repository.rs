use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};

use crate::db::models::{SplitSession, SplitShare, SplitType};

pub struct SplitRepository;

impl SplitRepository {
    pub async fn create_split_session_tx(
        tx: &mut Transaction<'_, Postgres>,
        order_id: Uuid,
        total_amount: f64,
        split_type: SplitType,
        expires_at: DateTime<Utc>,
    ) -> Result<SplitSession, (StatusCode, String)> {
        sqlx::query_as::<_, SplitSession>(
            r#"
            INSERT INTO split_sessions (order_id, total_amount, split_type, status, expires_at)
            VALUES ($1, $2, $3, 'Pending'::split_status, $4)
            RETURNING id, order_id, total_amount::float8, split_type, status, expires_at, created_at
            "#,
        )
        .bind(order_id)
        .bind(total_amount)
        .bind(split_type)
        .bind(expires_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create split session: {}", e)))
    }

    pub async fn create_split_share_tx(
        tx: &mut Transaction<'_, Postgres>,
        split_session_id: Uuid,
        amount_due: f64,
        is_host_share: bool,
        guest_name: Option<String>,
        guest_email: Option<String>,
    ) -> Result<SplitShare, (StatusCode, String)> {
        sqlx::query_as::<_, SplitShare>(
            r#"
            INSERT INTO split_shares (split_session_id, amount_due, status, is_host_share, guest_name, guest_email)
            VALUES ($1, $2, 'Pending'::split_status, $3, $4, $5)
            RETURNING id, split_session_id, amount_due::float8, status, is_host_share, guest_name, guest_email, payment_token,
                      gateway_order_id, gateway_payment_id, paid_at, claimed_by_user_id, claimed_ticket_id,
                      claimed_at, created_at, pending_manual_refund
            "#,
        )
        .bind(split_session_id)
        .bind(amount_due)
        .bind(is_host_share)
        .bind(guest_name)
        .bind(guest_email)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create split share: {}", e)))
    }

    pub async fn get_split_share_public_details(
        pool: &sqlx::PgPool,
        token: Uuid,
    ) -> Result<crate::db::models::SplitSharePublicDetail, (StatusCode, String)> {
        sqlx::query_as::<_, crate::db::models::SplitSharePublicDetail>(
            r#"
            SELECT 
                s.id,
                o.id AS order_id,
                s.amount_due::float8,
                s.status,
                o.user_id AS host_user_id,
                s.is_host_share,
                s.guest_name,
                s.guest_email,
                s.payment_token,
                s.gateway_order_id,
                e.title AS event_title,
                e.start_time AS event_start_time,
                e.venue_name,
                u.name AS host_name,
                s.claimed_ticket_id,
                s.claimed_at,
                ss.expires_at AS session_expires_at,
                ss.status AS session_status
            FROM split_shares s
            JOIN split_sessions ss ON ss.id = s.split_session_id
            JOIN orders o ON o.id = ss.order_id
            JOIN events e ON e.id = o.event_id
            JOIN users u ON u.id = o.user_id
            WHERE s.payment_token = $1
            "#
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?
        .ok_or((StatusCode::NOT_FOUND, "Token invalid or expired.".to_string()))
    }

    pub async fn get_share_by_payment_token(
        pool: &sqlx::PgPool,
        token: Uuid,
    ) -> Result<SplitShare, (StatusCode, String)> {
        sqlx::query_as::<_, SplitShare>(
            r#"
            SELECT id, split_session_id, amount_due::float8, status, guest_name, guest_email, payment_token,
                   is_host_share,
                   gateway_order_id, gateway_payment_id, paid_at, claimed_by_user_id, claimed_ticket_id,
                   claimed_at, created_at, pending_manual_refund
            FROM split_shares
            WHERE payment_token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?
        .ok_or((StatusCode::NOT_FOUND, "Token invalid or expired.".to_string()))
    }

    pub async fn attach_gateway_to_share_tx(
        tx: &mut Transaction<'_, Postgres>,
        token: Uuid,
        gateway_order_id: &str,
    ) -> Result<crate::db::models::SplitShare, (StatusCode, String)> {
        sqlx::query_as::<_, crate::db::models::SplitShare>(
            r#"
            UPDATE split_shares
            SET gateway_order_id = $2
            WHERE payment_token = $1
            RETURNING id, split_session_id, amount_due::float8, status, is_host_share, guest_name, guest_email, payment_token,
                      gateway_order_id, gateway_payment_id, paid_at, claimed_by_user_id, claimed_ticket_id,
                      claimed_at, created_at, pending_manual_refund
            "#
        )
        .bind(token)
        .bind(gateway_order_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))
    }

    pub async fn get_split_session_for_order_with_shares(
        pool: &sqlx::PgPool,
        order_id: Uuid,
    ) -> Result<crate::db::models::SplitSession, (StatusCode, String)> {
        let mut session = sqlx::query_as::<_, crate::db::models::SplitSession>(
            r#"
            SELECT id, order_id, total_amount::float8, split_type, status, expires_at, created_at
            FROM split_sessions
            WHERE order_id = $1
            "#
        )
        .bind(order_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?
        .ok_or((StatusCode::NOT_FOUND, "No split session found for this order.".to_string()))?;

        let shares = sqlx::query_as::<_, crate::db::models::SplitShare>(
            r#"
            SELECT id, split_session_id, amount_due::float8, status, is_host_share, guest_name, guest_email, payment_token,
                   gateway_order_id, gateway_payment_id, paid_at, claimed_by_user_id, claimed_ticket_id,
                   claimed_at, created_at, pending_manual_refund
            FROM split_shares
            WHERE split_session_id = $1
            ORDER BY is_host_share DESC, created_at ASC, id ASC
            "#
        )
        .bind(session.id)
        .fetch_all(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

        session.shares = Some(shares);
        Ok(session)
    }

    pub async fn find_expired_pending_sessions(
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Vec<(Uuid, Uuid)>, (StatusCode, String)> {
        sqlx::query_as::<_, (Uuid, Uuid)>(
            r#"
            SELECT id, order_id FROM split_sessions
            WHERE expires_at < NOW() AND status = 'Pending'
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to find expired pending sessions: {}", e)))
    }

    pub async fn get_shares_for_session(
        pool: &sqlx::PgPool,
        split_session_id: Uuid,
    ) -> Result<Vec<SplitShare>, (StatusCode, String)> {
        sqlx::query_as::<_, SplitShare>(
            r#"
            SELECT id, split_session_id, amount_due::float8, status, is_host_share, guest_name, guest_email, payment_token,
                   gateway_order_id, gateway_payment_id, paid_at, claimed_by_user_id, claimed_ticket_id,
                   claimed_at, created_at, pending_manual_refund
            FROM split_shares
            WHERE split_session_id = $1
            "#,
        )
        .bind(split_session_id)
        .fetch_all(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get shares for session: {}", e)))
    }

    pub async fn create_share_item_allocation_tx(
        tx: &mut Transaction<'_, Postgres>,
        split_share_id: Uuid,
        order_item_id: Uuid,
    ) -> Result<(), (StatusCode, String)> {
        sqlx::query(
            r#"
            INSERT INTO split_share_order_item_allocations (split_share_id, order_item_id)
            VALUES ($1, $2)
            "#,
        )
        .bind(split_share_id)
        .bind(order_item_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to allocate split share item: {}", e)))?;

        Ok(())
    }
}
