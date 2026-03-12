use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use axum::http::StatusCode;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::db::models::{Ticket, TicketDetail};

type HmacSha256 = Hmac<Sha256>;

pub struct TicketRepository;

impl TicketRepository {
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

        // Insert ticket seats
        sqlx::query(
            r#"
            INSERT INTO ticket_seats (ticket_id, seat_id)
            SELECT $1, seat_id FROM UNNEST($2::uuid[]) as seat_id
            "#
        )
        .bind(ticket.id)
        .bind(seat_ids)
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
        sqlx::query_as::<_, TicketDetail>(
            r#"
            SELECT
                t.id,
                t.order_id,
                t.event_id,
                e.title          AS event_title,
                e.start_time     AS event_start_time,
                e.venue_name,
                (
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
                ) AS seats,
                (t.id::text || ':' || t.qr_secret) AS qr_payload,
                t.status,
                t.created_at,
                t.used_at
            FROM tickets t
            JOIN events e       ON e.id  = t.event_id
            WHERE t.user_id = $1
            ORDER BY t.created_at DESC
            "#,
        )
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
        sqlx::query_as::<_, TicketDetail>(
            r#"
            SELECT
                t.id,
                t.order_id,
                t.event_id,
                e.title          AS event_title,
                e.start_time     AS event_start_time,
                e.venue_name,
                (
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
                ) AS seats,
                (t.id::text || ':' || t.qr_secret) AS qr_payload,
                t.status,
                t.created_at,
                t.used_at
            FROM tickets t
            JOIN events e       ON e.id  = t.event_id
            WHERE t.id = $1 AND t.user_id = $2
            "#,
        )
        .bind(ticket_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Ticket not found.".to_string()))
    }
}
