use crate::constants::order_status;
use crate::db::models::Order;
use crate::repositories::ticket_repository::TicketRepository;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct OrderRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HeldItem {
    pub hold_id: Uuid,
    pub seat_id: Option<Uuid>,
    pub ticket_tier_id: Option<Uuid>,
    pub price: f64,
}

impl OrderRepository {
    const ORDER_COLUMNS: &'static str = r#"
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
    "#;

    fn order_select_query(where_clause: &str) -> String {
        format!(
            "SELECT {} FROM orders {}",
            Self::ORDER_COLUMNS,
            where_clause
        )
    }

    fn order_update_returning_query(set_clause: &str, where_clause: &str) -> String {
        format!(
            "UPDATE orders SET {} {} RETURNING {}",
            set_clause,
            where_clause,
            Self::ORDER_COLUMNS
        )
    }

    pub async fn get_active_held_items(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        user_id: Uuid,
        hold_ids: &[Uuid],
    ) -> Result<Vec<HeldItem>, (StatusCode, String)> {
        let held_items = sqlx::query_as::<_, HeldItem>(
            r#"
            SELECT
                sh.id AS hold_id,
                sh.seat_id,
                sh.ticket_tier_id,
                COALESCE(ett.price::float8, esc.price::float8, 0.0) AS price
            FROM seat_holds sh
            LEFT JOIN event_ticket_tiers ett ON ett.id = sh.ticket_tier_id
            LEFT JOIN venue_seats vs ON vs.id = sh.seat_id
            LEFT JOIN venue_rows vr ON vr.id = vs.row_id
            LEFT JOIN event_seat_categories esc
              ON esc.event_id = sh.event_id
             AND esc.section_id = vr.section_id
            WHERE sh.event_id = $1
              AND sh.id = ANY($2)
              AND sh.user_id = $3
              AND sh.expires_at > NOW()
            "#,
        )
        .bind(event_id)
        .bind(hold_ids)
        .bind(user_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if held_items.len() != hold_ids.len() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Some holds are no longer active.".to_string(),
            ));
        }

        Ok(held_items)
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
        let query = format!(
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
            VALUES ($1, $2, $3, $4, $5, $6, '{}')
            RETURNING {}
            "#,
            order_status::PENDING,
            Self::ORDER_COLUMNS,
        );

        sqlx::query_as::<_, Order>(&query)
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
        held_items: &[HeldItem],
    ) -> Result<(), (StatusCode, String)> {
        for item in held_items {
            sqlx::query(
                r#"
                INSERT INTO order_items (order_id, seat_id, ticket_tier_id, price)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(order_id)
            .bind(item.seat_id)
            .bind(item.ticket_tier_id)
            .bind(item.price)
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
        hold_ids: &[Uuid],
        expires_at: DateTime<Utc>,
    ) -> Result<(), (StatusCode, String)> {
        let result = sqlx::query(
            r#"
            UPDATE seat_holds
            SET expires_at = $4
            WHERE event_id = $1
              AND user_id = $2
              AND id = ANY($3)
              AND expires_at > NOW()
            "#,
        )
        .bind(event_id)
        .bind(user_id)
        .bind(hold_ids)
        .bind(expires_at)
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.rows_affected() != hold_ids.len() as u64 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Some ticket holds expired before payment could begin.".to_string(),
            ));
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
        let query = Self::order_update_returning_query(
            "gateway = $2, gateway_order_id = $3, receipt = $4, failure_reason = NULL",
            "WHERE id = $1",
        );

        sqlx::query_as::<_, Order>(&query)
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
        let query = Self::order_select_query("WHERE id = $1 AND user_id = $2");

        sqlx::query_as::<_, Order>(&query)
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
        let query = Self::order_select_query("WHERE gateway_order_id = $1");

        sqlx::query_as::<_, Order>(&query)
            .bind(gateway_order_id)
            .fetch_one(pool)
            .await
            .map_err(|e: sqlx::Error| match e {
                sqlx::Error::RowNotFound => (
                    StatusCode::NOT_FOUND,
                    "Order not found for gateway order.".to_string(),
                ),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            })
    }

    pub async fn list_user_orders(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Order>, (StatusCode, String)> {
        let query = Self::order_select_query("WHERE user_id = $1 ORDER BY created_at DESC");

        sqlx::query_as::<_, Order>(&query)
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
        let query =
            Self::order_select_query("WHERE id = $1 AND user_id = $2 AND event_id = $3 FOR UPDATE");

        let order = sqlx::query_as::<_, Order>(&query)
            .bind(order_id)
            .bind(user_id)
            .bind(event_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| match e {
                sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Order not found.".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            })?;

        if order.status == order_status::COMPLETED {
            if order.gateway_payment_id.as_deref() == Some(gateway_payment_id) {
                return Ok(order);
            }

            return Err((
                StatusCode::CONFLICT,
                "Order was already paid with a different payment id.".to_string(),
            ));
        }

        if order.status != order_status::PENDING && order.status != order_status::FAILED {
            return Err((
                StatusCode::CONFLICT,
                format!(
                    "Order is not payable in its current state: {}.",
                    order.status
                ),
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT seat_id, ticket_tier_id
            FROM order_items
            WHERE order_id = $1
            ORDER BY id
            "#,
        )
        .bind(order.id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut ticket_seat_ids = Vec::new();
        let mut ticket_tier_quantities: BTreeMap<Uuid, i32> = BTreeMap::new();
        for row in rows {
            let seat_id = row
                .try_get::<Option<Uuid>, _>("seat_id")
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let ticket_tier_id = row
                .try_get::<Option<Uuid>, _>("ticket_tier_id")
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            match (seat_id, ticket_tier_id) {
                (Some(seat_id), None) => ticket_seat_ids.push(seat_id),
                (None, Some(ticket_tier_id)) => {
                    *ticket_tier_quantities.entry(ticket_tier_id).or_insert(0) += 1;
                }
                _ => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Order item is missing its purchase target.".to_string(),
                    ));
                }
            }
        }

        if ticket_seat_ids.is_empty() && ticket_tier_quantities.is_empty() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Order has no purchasable items.".to_string(),
            ));
        }

        if !ticket_seat_ids.is_empty() {
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
                return Err((
                    StatusCode::CONFLICT,
                    "Seat reservation expired while payment was being verified.".to_string(),
                ));
            }
        }

        for (ticket_tier_id, quantity) in &ticket_tier_quantities {
            let held_count = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM seat_holds
                WHERE event_id = $1
                  AND user_id = $2
                  AND ticket_tier_id = $3
                  AND expires_at > NOW()
                "#,
            )
            .bind(event_id)
            .bind(user_id)
            .bind(ticket_tier_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if held_count < i64::from(*quantity) {
                return Err((
                    StatusCode::CONFLICT,
                    "Ticket hold expired while payment was being verified.".to_string(),
                ));
            }
        }

        TicketRepository::create_ticket_in_tx(
            tx,
            order.id,
            event_id,
            &ticket_seat_ids,
            &ticket_tier_quantities
                .iter()
                .map(|(ticket_tier_id, quantity)| (*ticket_tier_id, *quantity))
                .collect::<Vec<_>>(),
            user_id,
            jwt_secret,
        )
        .await?;

        if !ticket_seat_ids.is_empty() {
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
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update seat inventory status.".to_string(),
                ));
            }
        }

        if !ticket_seat_ids.is_empty() {
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
        }

        for (ticket_tier_id, quantity) in ticket_tier_quantities {
            sqlx::query(
                r#"
                DELETE FROM seat_holds
                WHERE id IN (
                    SELECT id
                    FROM seat_holds
                    WHERE event_id = $1
                      AND user_id = $2
                      AND ticket_tier_id = $3
                      AND expires_at > NOW()
                    ORDER BY created_at ASC
                    LIMIT $4
                )
                "#,
            )
            .bind(event_id)
            .bind(user_id)
            .bind(ticket_tier_id)
            .bind(i64::from(quantity))
            .execute(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let query = Self::order_update_returning_query(
            &format!(
                "status = '{}', gateway_payment_id = $2, payment_signature = $3, payment_verified_at = NOW(), failure_reason = NULL",
                order_status::COMPLETED
            ),
            "WHERE id = $1",
        );

        sqlx::query_as::<_, Order>(&query)
            .bind(order.id)
            .bind(gateway_payment_id)
            .bind(payment_signature)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn finalize_split_order(
        tx: &mut Transaction<'_, Postgres>,
        order_id: Uuid,
        event_id: Uuid,
        user_id: Uuid,
        jwt_secret: &str,
    ) -> Result<Order, (StatusCode, String)> {
        let query =
            Self::order_select_query("WHERE id = $1 AND user_id = $2 AND event_id = $3 FOR UPDATE");

        let order = sqlx::query_as::<_, Order>(&query)
            .bind(order_id)
            .bind(user_id)
            .bind(event_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| match e {
                sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Order not found.".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            })?;

        if order.status == order_status::COMPLETED {
            return Ok(order);
        }

        if order.status != order_status::PENDING && order.status != order_status::FAILED {
            return Err((
                StatusCode::CONFLICT,
                format!(
                    "Order is not payable in its current state: {}.",
                    order.status
                ),
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT seat_id, ticket_tier_id
            FROM order_items
            WHERE order_id = $1
            ORDER BY id
            "#,
        )
        .bind(order.id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut ticket_seat_ids = Vec::new();
        let mut ticket_tier_quantities: BTreeMap<Uuid, i32> = BTreeMap::new();
        for row in rows {
            let seat_id = row
                .try_get::<Option<Uuid>, _>("seat_id")
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let ticket_tier_id = row
                .try_get::<Option<Uuid>, _>("ticket_tier_id")
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            match (seat_id, ticket_tier_id) {
                (Some(seat_id), None) => ticket_seat_ids.push(seat_id),
                (None, Some(ticket_tier_id)) => {
                    *ticket_tier_quantities.entry(ticket_tier_id).or_insert(0) += 1;
                }
                _ => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Order item is missing its purchase target.".to_string(),
                    ));
                }
            }
        }

        if ticket_seat_ids.is_empty() && ticket_tier_quantities.is_empty() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Order has no purchasable items.".to_string(),
            ));
        }

        // NOTE: Hold-expiry checks are intentionally skipped here.
        // Holds are already extended to 30 minutes at split creation time,
        // and payment has just completed — no need to re-validate expiry.

        TicketRepository::create_ticket_in_tx(
            tx,
            order.id,
            event_id,
            &ticket_seat_ids,
            &ticket_tier_quantities
                .iter()
                .map(|(ticket_tier_id, quantity)| (*ticket_tier_id, *quantity))
                .collect::<Vec<_>>(),
            user_id,
            jwt_secret,
        )
        .await?;

        if !ticket_seat_ids.is_empty() {
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
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update seat inventory status.".to_string(),
                ));
            }
        }

        if !ticket_seat_ids.is_empty() {
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
        }

        for (ticket_tier_id, quantity) in ticket_tier_quantities {
            sqlx::query(
                r#"
                DELETE FROM seat_holds
                WHERE id IN (
                    SELECT id
                    FROM seat_holds
                    WHERE event_id = $1
                      AND user_id = $2
                      AND ticket_tier_id = $3
                      AND expires_at > NOW()
                    ORDER BY created_at ASC
                    LIMIT $4
                )
                "#,
            )
            .bind(event_id)
            .bind(user_id)
            .bind(ticket_tier_id)
            .bind(i64::from(quantity))
            .execute(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let query = Self::order_update_returning_query(
            &format!(
                "status = '{}', payment_verified_at = NOW(), failure_reason = NULL",
                order_status::COMPLETED
            ),
            "WHERE id = $1",
        );

        sqlx::query_as::<_, Order>(&query)
            .bind(order.id)
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
