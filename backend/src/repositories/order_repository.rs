use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use axum::http::StatusCode;
use crate::db::models::Order;
use crate::repositories::ticket_repository::TicketRepository;

pub struct OrderRepository;

impl OrderRepository {
    pub async fn create_checkout_transaction(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        user_id: Uuid,
        seat_ids: &[Uuid],
        jwt_secret: &str,
    ) -> Result<Order, (StatusCode, String)> {
        // 1. Validate that holds exist and are owned by the current user, and haven't expired
        #[derive(sqlx::FromRow)]
        struct HeldSeat {
            seat_id: Uuid,
            price: f64,
        }

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

        let total_amount: f64 = held_seats.iter().map(|s| s.price).sum();

        // 2. Insert Order
        let order = sqlx::query_as!(
            Order,
            r#"
            INSERT INTO orders (user_id, event_id, total_amount, status)
            VALUES ($1, $2, $3, 'Completed')
            RETURNING id, user_id, event_id, total_amount::float8 as "total_amount!", status, created_at
            "#,
            user_id,
            event_id,
            total_amount
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // 3. Insert Order Items and generate Tickets
        for seat in &held_seats {
            let order_item = sqlx::query_as::<_, crate::db::models::OrderItem>(
                r#"
                INSERT INTO order_items (order_id, seat_id, price)
                VALUES ($1, $2, $3)
                RETURNING id, order_id, seat_id, price
                "#,
            )
            .bind(order.id)
            .bind(seat.seat_id)
            .bind(seat.price as f64)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            // Generate ticket for this order item
            TicketRepository::create_ticket_in_tx(
                tx,
                order.id,
                order_item.id,
                event_id,
                seat.seat_id,
                user_id,
                jwt_secret,
            )
            .await?;
        }

        // 4. Update Inventory Status to Sold
        let result = sqlx::query!(
            r#"
            UPDATE event_seat_inventory
            SET status = 'Sold'::seat_status
            WHERE event_id = $1 AND seat_id = ANY($2)
            "#,
            event_id,
            seat_ids
        )
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.rows_affected() != seat_ids.len() as u64 {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update seat inventory status.".to_string()));
        }

        // 5. Release Holds
        sqlx::query!(
            r#"
            DELETE FROM seat_holds
            WHERE event_id = $1 AND seat_id = ANY($2)
            "#,
            event_id,
            seat_ids
        )
        .execute(&mut **tx)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(order)
    }

    pub async fn list_user_orders(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Order>, (StatusCode, String)> {
        sqlx::query_as!(
            Order,
            r#"
            SELECT id, user_id, event_id, total_amount::float8 as "total_amount!", status, created_at
            FROM orders
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}
