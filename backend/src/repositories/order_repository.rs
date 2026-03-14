use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;
use axum::http::StatusCode;
use crate::db::models::Order;
use crate::repositories::ticket_repository::TicketRepository;

pub struct OrderRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HeldSeat {
    pub seat_id: Uuid,
    pub price: f64,
}

impl OrderRepository {
    pub async fn get_active_held_seats(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        user_id: Uuid,
        seat_ids: &[Uuid],
    ) -> Result<Vec<HeldSeat>, (StatusCode, String)> {
        let held_seats = sqlx::query_as::<_, HeldSeat>(
            r#"
            SELECT esi.seat_id, COALESCE(esc.price::float8, 0.0) as price
            FROM event_seat_inventory esi
            JOIN seat_holds sh ON esi.seat_id = sh.seat_id AND esi.event_id = sh.event_id
            JOIN venue_seats vs ON vs.id = esi.seat_id
            LEFT JOIN event_seat_categories esc ON esc.event_id = esi.event_id AND esc.section_id = (SELECT section_id FROM venue_rows WHERE id = vs.row_id)
            WHERE esi.event_id = $1 
              AND esi.seat_id = ANY($2)
              AND esi.status = 'Held'::seat_status
              AND sh.user_id = $3
              AND sh.expires_at > NOW()
            "#,
        )
        .bind(event_id)
        .bind(seat_ids)
        .bind(user_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if held_seats.len() != seat_ids.len() {
            return Err((StatusCode::BAD_REQUEST, "Some seats are not held by you or holds have expired.".to_string()));
        }

        Ok(held_seats)
    }

    pub async fn create_pending_order(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        user_id: Uuid,
        subtotal_amount: f64,
        fee_amount: f64,
        total_amount: f64,
        currency: &str,
    ) -> Result<Order, (StatusCode, String)> {
        sqlx::query_as::<_, Order>(
            r#"
            INSERT INTO orders (
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'Pending')
            RETURNING
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            "#,
        )
        .bind(user_id)
        .bind(event_id)
        .bind(subtotal_amount)
        .bind(fee_amount)
        .bind(total_amount)
        .bind(currency)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn create_order_items(
        tx: &mut Transaction<'_, Postgres>,
        order_id: Uuid,
        held_seats: &[HeldSeat],
    ) -> Result<(), (StatusCode, String)> {
        for seat in held_seats {
            sqlx::query(
                r#"
                INSERT INTO order_items (order_id, seat_id, price)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(order_id)
            .bind(seat.seat_id)
            .bind(seat.price)
            .execute(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        Ok(())
    }

    pub async fn extend_hold_expiry(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        user_id: Uuid,
        seat_ids: &[Uuid],
        expires_at: DateTime<Utc>,
    ) -> Result<(), (StatusCode, String)> {
        let result = sqlx::query(
            r#"
            UPDATE seat_holds
            SET expires_at = $4
            WHERE event_id = $1
              AND user_id = $2
              AND seat_id = ANY($3)
              AND expires_at > NOW()
            "#,
        )
        .bind(event_id)
        .bind(user_id)
        .bind(seat_ids)
        .bind(expires_at)
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.rows_affected() != seat_ids.len() as u64 {
            return Err((StatusCode::BAD_REQUEST, "Some seat holds expired before payment could begin.".to_string()));
        }

        Ok(())
    }

    pub async fn attach_gateway_order(
        pool: &PgPool,
        order_id: Uuid,
        gateway: &str,
        gateway_order_id: &str,
        receipt: &str,
    ) -> Result<Order, (StatusCode, String)> {
        sqlx::query_as::<_, Order>(
            r#"
            UPDATE orders
            SET gateway = $2,
                gateway_order_id = $3,
                receipt = $4,
                failure_reason = NULL
            WHERE id = $1
            RETURNING
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            "#,
        )
        .bind(order_id)
        .bind(gateway)
        .bind(gateway_order_id)
        .bind(receipt)
        .fetch_one(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn mark_order_failed(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
        failure_reason: Option<&str>,
        gateway_payment_id: Option<&str>,
    ) -> Result<(), (StatusCode, String)> {
        let result = sqlx::query(
            r#"
            UPDATE orders
            SET status = CASE WHEN status = 'Completed' THEN status ELSE 'Failed' END,
                failure_reason = COALESCE($3, failure_reason),
                gateway_payment_id = COALESCE($4, gateway_payment_id)
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(order_id)
        .bind(user_id)
        .bind(failure_reason)
        .bind(gateway_payment_id)
        .execute(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err((StatusCode::NOT_FOUND, "Order not found.".to_string()));
        }

        Ok(())
    }

    pub async fn mark_order_manual_review(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
        gateway_payment_id: &str,
        payment_signature: &str,
        failure_reason: &str,
    ) -> Result<(), (StatusCode, String)> {
        sqlx::query(
            r#"
            UPDATE orders
            SET status = 'ManualReview',
                gateway_payment_id = $3,
                payment_signature = $4,
                payment_verified_at = NOW(),
                failure_reason = $5
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(order_id)
        .bind(user_id)
        .bind(gateway_payment_id)
        .bind(payment_signature)
        .bind(failure_reason)
        .execute(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn get_order_by_id_for_user(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
    ) -> Result<Order, (StatusCode, String)> {
        sqlx::query_as::<_, Order>(
            r#"
            SELECT
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            FROM orders
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e: sqlx::Error| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Order not found.".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })
    }

    pub async fn get_order_by_gateway_order_id(
        pool: &PgPool,
        gateway_order_id: &str,
    ) -> Result<Order, (StatusCode, String)> {
        sqlx::query_as::<_, Order>(
            r#"
            SELECT
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            FROM orders
            WHERE gateway_order_id = $1
            "#,
        )
        .bind(gateway_order_id)
        .fetch_one(pool)
        .await
        .map_err(|e: sqlx::Error| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Order not found for gateway order.".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })
    }

    pub async fn list_user_orders(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Order>, (StatusCode, String)> {
        sqlx::query_as::<_, Order>(
            r#"
            SELECT
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            FROM orders
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn finalize_checkout_transaction(
        tx: &mut Transaction<'_, Postgres>,
        order_id: Uuid,
        event_id: Uuid,
        user_id: Uuid,
        jwt_secret: &str,
        gateway_payment_id: &str,
        payment_signature: &str,
    ) -> Result<Order, (StatusCode, String)> {
        let order = sqlx::query_as::<_, Order>(
            r#"
            SELECT
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            FROM orders
            WHERE id = $1 AND user_id = $2 AND event_id = $3
            FOR UPDATE
            "#,
        )
        .bind(order_id)
        .bind(user_id)
        .bind(event_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Order not found.".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

        if order.status == "Completed" {
            if order.gateway_payment_id.as_deref() == Some(gateway_payment_id) {
                return Ok(order);
            }

            return Err((StatusCode::CONFLICT, "Order was already paid with a different payment id.".to_string()));
        }

        if order.status != "Pending" && order.status != "Failed" {
            return Err((StatusCode::CONFLICT, format!("Order is not payable in its current state: {}.", order.status)));
        }

        let rows = sqlx::query(
            r#"
            SELECT seat_id, price
            FROM order_items
            WHERE order_id = $1
            ORDER BY seat_id
            "#,
        )
        .bind(order.id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut ticket_seat_ids = Vec::with_capacity(rows.len());
        for row in rows {
            ticket_seat_ids.push(row.try_get::<Uuid, _>("seat_id").map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
        }

        if ticket_seat_ids.is_empty() {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Order has no seat items.".to_string()));
        }

        let held_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM seat_holds sh
            JOIN event_seat_inventory esi
              ON esi.event_id = sh.event_id
             AND esi.seat_id = sh.seat_id
            WHERE sh.event_id = $1
              AND sh.user_id = $2
              AND sh.seat_id = ANY($3)
              AND sh.expires_at > NOW()
              AND esi.status = 'Held'::seat_status
            "#,
        )
        .bind(event_id)
        .bind(user_id)
        .bind(&ticket_seat_ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if held_count != ticket_seat_ids.len() as i64 {
            return Err((StatusCode::CONFLICT, "Seat reservation expired while payment was being verified.".to_string()));
        }

        TicketRepository::create_ticket_in_tx(
            tx,
            order.id,
            event_id,
            &ticket_seat_ids,
            user_id,
            jwt_secret,
        )
        .await?;

        let result = sqlx::query(
            r#"
            UPDATE event_seat_inventory
            SET status = 'Sold'::seat_status
            WHERE event_id = $1 AND seat_id = ANY($2)
            "#,
        )
        .bind(event_id)
        .bind(&ticket_seat_ids)
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.rows_affected() != ticket_seat_ids.len() as u64 {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update seat inventory status.".to_string()));
        }

        sqlx::query(
            r#"
            DELETE FROM seat_holds
            WHERE event_id = $1 AND seat_id = ANY($2)
            "#,
        )
        .bind(event_id)
        .bind(&ticket_seat_ids)
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query_as::<_, Order>(
            r#"
            UPDATE orders
            SET status = 'Completed',
                gateway_payment_id = $2,
                payment_signature = $3,
                payment_verified_at = NOW(),
                failure_reason = NULL
            WHERE id = $1
            RETURNING
                id,
                user_id,
                event_id,
                subtotal_amount,
                fee_amount,
                total_amount,
                currency,
                status,
                gateway,
                gateway_order_id,
                gateway_payment_id,
                payment_signature,
                payment_verified_at,
                receipt,
                failure_reason,
                created_at
            "#,
        )
        .bind(order.id)
        .bind(gateway_payment_id)
        .bind(payment_signature)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn record_webhook_event(
        pool: &PgPool,
        event_id: &str,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<bool, (StatusCode, String)> {
        let result = sqlx::query(
            r#"
            INSERT INTO payment_webhook_events (provider_event_id, event_type, payload)
            VALUES ($1, $2, $3::jsonb)
            ON CONFLICT (provider_event_id) DO NOTHING
            "#,
        )
        .bind(event_id)
        .bind(event_type)
        .bind(payload)
        .execute(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(result.rows_affected() == 1)
    }
}
