use axum::http::StatusCode;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::constants::{order_status, ticket_status};
use crate::db::models::{EventStatus, Ticket, TicketDetail};

type HmacSha256 = Hmac<Sha256>;

pub struct TicketRepository;

impl TicketRepository {
    const TICKET_DETAIL_SELECT: &'static str = r#"
        SELECT
            t.id,
            t.order_id,
            t.event_id,
            e.title          AS event_title,
            e.start_time     AS event_start_time,
            e.venue_name,
            COALESCE((
                SELECT json_agg(json_build_object(
                    'seat_id', vs.id,
                    'seat_label', vs.seat_label,
                    'section_name', sec.name
                ))
                FROM ticket_seats ts
                JOIN venue_seats vs ON vs.id = ts.seat_id
                JOIN venue_rows vr  ON vr.id = vs.row_id
                JOIN venue_sections sec ON sec.id = vr.section_id
                WHERE ts.ticket_id = t.id
            ), '[]'::json) AS seats,
            COALESCE((
                SELECT json_agg(json_build_object(
                    'ticket_tier_id', ett.id,
                    'name', ett.name,
                    'quantity', tt.quantity,
                    'price', ett.price,
                    'color_hex', ett.color_hex
                ))
                FROM ticket_tiers tt
                JOIN event_ticket_tiers ett ON ett.id = tt.ticket_tier_id
                WHERE tt.ticket_id = t.id
            ), '[]'::json) AS tiers,
            (t.id::text || ':' || t.qr_secret) AS qr_payload,
            t.status,
            (t.status = 'Valid' AND e.start_time > NOW() AND e.status <> 'Cancelled') AS can_cancel,
            t.created_at,
            t.used_at
        FROM tickets t
        JOIN events e ON e.id = t.event_id
    "#;

    /// Generate HMAC-SHA256 QR secret for a ticket ID.
    pub fn generate_qr_secret(ticket_id: &Uuid, jwt_secret: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(jwt_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(ticket_id.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Create a single ticket for an order inside an existing transaction.
    /// Called once per checkout.
    pub async fn create_ticket_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        order_id: Uuid,
        event_id: Uuid,
        seat_ids: &[Uuid],
        ticket_tiers: &[(Uuid, i32)],
        user_id: Uuid,
        jwt_secret: &str,
    ) -> Result<Ticket, (StatusCode, String)> {
        // First insert with a placeholder qr_secret to get the ticket ID
        let ticket = sqlx::query_as::<_, Ticket>(
            r#"
            INSERT INTO tickets (order_id, event_id, user_id, qr_secret)
            VALUES ($1, $2, $3, 'placeholder')
            RETURNING id, order_id, event_id, user_id,
                      qr_secret, status, created_at, used_at
            "#,
        )
        .bind(order_id)
        .bind(event_id)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if !seat_ids.is_empty() {
            sqlx::query(
                r#"
                INSERT INTO ticket_seats (ticket_id, seat_id)
                SELECT $1, seat_id FROM UNNEST($2::uuid[]) as seat_id
                "#,
            )
            .bind(ticket.id)
            .bind(seat_ids)
            .execute(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        for (ticket_tier_id, quantity) in ticket_tiers {
            sqlx::query(
                r#"
                INSERT INTO ticket_tiers (ticket_id, ticket_tier_id, quantity)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(ticket.id)
            .bind(ticket_tier_id)
            .bind(quantity)
            .execute(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        // Now compute the real HMAC-based qr_secret using the generated ticket ID
        let qr_secret = Self::generate_qr_secret(&ticket.id, jwt_secret);

        let ticket = sqlx::query_as::<_, Ticket>(
            r#"
            UPDATE tickets SET qr_secret = $1 WHERE id = $2
            RETURNING id, order_id, event_id, user_id,
                      qr_secret, status, created_at, used_at
            "#,
        )
        .bind(&qr_secret)
        .bind(ticket.id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(ticket)
    }

    /// List all tickets for a user with rich event/seat details.
    pub async fn list_user_tickets(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<TicketDetail>, (StatusCode, String)> {
        let query = format!(
            "{} WHERE t.user_id = $1 ORDER BY t.created_at DESC",
            Self::TICKET_DETAIL_SELECT
        );

        sqlx::query_as::<_, TicketDetail>(&query)
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    /// Get a single ticket by ID, only if owned by the given user.
    pub async fn get_ticket_by_id(
        pool: &PgPool,
        ticket_id: Uuid,
        user_id: Uuid,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        let query = format!(
            "{} WHERE t.id = $1 AND t.user_id = $2",
            Self::TICKET_DETAIL_SELECT
        );

        sqlx::query_as::<_, TicketDetail>(&query)
            .bind(ticket_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Ticket not found.".to_string()))
    }

    pub async fn get_ticket_detail_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        ticket_id: Uuid,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        let query = format!("{} WHERE t.id = $1", Self::TICKET_DETAIL_SELECT);

        sqlx::query_as::<_, TicketDetail>(&query)
            .bind(ticket_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::NOT_FOUND,
                "Ticket details not found".to_string(),
            ))
    }

    pub async fn cancel_ticket(
        pool: &PgPool,
        ticket_id: Uuid,
        user_id: Uuid,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        #[derive(sqlx::FromRow)]
        struct TicketCancellationCandidate {
            order_id: Uuid,
            event_id: Uuid,
            status: String,
            start_time: chrono::DateTime<chrono::Utc>,
            event_status: EventStatus,
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let candidate = sqlx::query_as::<_, TicketCancellationCandidate>(
            r#"
            SELECT
                t.order_id,
                t.event_id,
                t.status,
                e.start_time,
                e.status AS event_status
            FROM tickets t
            JOIN events e ON e.id = t.event_id
            WHERE t.id = $1 AND t.user_id = $2
            FOR UPDATE
            "#,
        )
        .bind(ticket_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Ticket not found.".to_string()))?;

        if candidate.status != ticket_status::VALID {
            tx.rollback()
                .await
                .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Err((
                StatusCode::CONFLICT,
                "Only valid tickets can be cancelled.".to_string(),
            ));
        }

        if candidate.start_time <= Utc::now() {
            tx.rollback()
                .await
                .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Err((
                StatusCode::CONFLICT,
                "This ticket can no longer be cancelled.".to_string(),
            ));
        }

        if candidate.event_status == EventStatus::Cancelled {
            tx.rollback()
                .await
                .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Err((
                StatusCode::CONFLICT,
                "This event has already been cancelled by the organizer.".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE tickets
            SET status = 'Cancelled',
                used_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(ticket_id)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE event_seat_inventory
            SET status = 'Available'::seat_status
            WHERE event_id = $1
              AND seat_id IN (SELECT seat_id FROM ticket_seats WHERE ticket_id = $2)
              AND status = 'Sold'::seat_status
            "#,
        )
        .bind(candidate.event_id)
        .bind(ticket_id)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query(
            r#"
            DELETE FROM seat_holds
            WHERE event_id = $1
              AND seat_id IN (SELECT seat_id FROM ticket_seats WHERE ticket_id = $2)
            "#,
        )
        .bind(candidate.event_id)
        .bind(ticket_id)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE orders
            SET status = CASE
                    WHEN status = $2 THEN $3
                    WHEN status = $3 THEN status
                    ELSE $4
                END,
                failure_reason = COALESCE(failure_reason, 'Cancelled by customer')
            WHERE id = $1
            "#,
        )
        .bind(candidate.order_id)
        .bind(order_status::COMPLETED)
        .bind(order_status::REFUNDED)
        .bind(order_status::CANCELLED)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Self::get_ticket_by_id(pool, ticket_id, user_id).await
    }
}
