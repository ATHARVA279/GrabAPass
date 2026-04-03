use crate::db::models::SeatHold;
use axum::http::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub struct HoldRepository;

impl HoldRepository {
    pub async fn create_hold_transaction(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        seat_id: Uuid,
        user_id: Uuid,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<SeatHold, (StatusCode, String)> {
        // 1. Check if the seat is Available and mark it as Held
        // 1. Check if the seat is Available OR if it's Held but expired. Mark it as Held.
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE event_seat_inventory esi
            SET status = 'Held'::seat_status
            WHERE esi.event_id = $1 AND esi.seat_id = $2 
              AND (
                esi.status = 'Available'::seat_status
                OR
                (esi.status = 'Held'::seat_status AND EXISTS (
                    SELECT 1 FROM seat_holds sh 
                    WHERE sh.seat_id = $2 AND sh.event_id = $1 AND sh.expires_at <= NOW()
                ))
              )
            RETURNING esi.id
            "#,
        )
        .bind(event_id)
        .bind(seat_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.is_none() {
            return Err((
                StatusCode::BAD_REQUEST,
                "One or more seats are no longer available.".to_string(),
            ));
        }

        // 2. Reuse an existing hold row when present, otherwise insert a new one.
        let hold = match sqlx::query_as::<_, SeatHold>(
            r#"
            UPDATE seat_holds
            SET user_id = $3,
                expires_at = $4,
                created_at = NOW()
            WHERE event_id = $1
              AND seat_id = $2
              AND ticket_tier_id IS NULL
            RETURNING id, event_id, seat_id, ticket_tier_id, user_id, created_at, expires_at
            "#,
        )
        .bind(event_id)
        .bind(seat_id)
        .bind(user_id)
        .bind(expires_at)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            Some(hold) => hold,
            None => sqlx::query_as::<_, SeatHold>(
                r#"
                    INSERT INTO seat_holds (event_id, seat_id, ticket_tier_id, user_id, expires_at)
                    VALUES ($1, $2, NULL, $3, $4)
                    RETURNING id, event_id, seat_id, ticket_tier_id, user_id, created_at, expires_at
                    "#,
            )
            .bind(event_id)
            .bind(seat_id)
            .bind(user_id)
            .bind(expires_at)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        };

        Ok(hold)
    }

    pub async fn create_ga_holds_transaction(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        ticket_tier_id: Uuid,
        user_id: Uuid,
        quantity: i32,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<SeatHold>, (StatusCode, String)> {
        let tier_capacity = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT capacity
            FROM event_ticket_tiers
            WHERE id = $1 AND event_id = $2
            FOR UPDATE
            "#,
        )
        .bind(ticket_tier_id)
        .bind(event_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Ticket tier not found for this event.".to_string(),
        ))?;

        let held_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM seat_holds
            WHERE event_id = $1
              AND ticket_tier_id = $2
              AND expires_at > NOW()
            "#,
        )
        .bind(event_id)
        .bind(ticket_tier_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let sold_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(tt.quantity), 0)
            FROM ticket_tiers tt
            JOIN tickets t ON t.id = tt.ticket_id
            WHERE t.event_id = $1
              AND tt.ticket_tier_id = $2
              AND t.status <> 'Cancelled'
            "#,
        )
        .bind(event_id)
        .bind(ticket_tier_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let available = i64::from(tier_capacity) - held_count - sold_count;
        if available < i64::from(quantity) {
            return Err((
                StatusCode::CONFLICT,
                "Not enough tickets remain in this tier.".to_string(),
            ));
        }

        let mut holds = Vec::with_capacity(quantity as usize);
        for _ in 0..quantity {
            let hold = sqlx::query_as::<_, SeatHold>(
                r#"
                INSERT INTO seat_holds (event_id, seat_id, ticket_tier_id, user_id, expires_at)
                VALUES ($1, NULL, $2, $3, $4)
                RETURNING id, event_id, seat_id, ticket_tier_id, user_id, created_at, expires_at
                "#,
            )
            .bind(event_id)
            .bind(ticket_tier_id)
            .bind(user_id)
            .bind(expires_at)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            holds.push(hold);
        }

        Ok(holds)
    }

    pub async fn release_expired_holds(pool: &PgPool) -> Result<usize, (StatusCode, String)> {
        let released = sqlx::query_scalar::<_, i64>(
            r#"
            WITH expired_holds AS (
                DELETE FROM seat_holds
                WHERE expires_at <= NOW()
                RETURNING event_id, seat_id
            ),
            released_inventory AS (
                UPDATE event_seat_inventory esi
                SET status = 'Available'::seat_status
                WHERE esi.status = 'Held'::seat_status
                  AND EXISTS (
                      SELECT 1
                      FROM expired_holds eh
                      WHERE eh.event_id = esi.event_id
                        AND eh.seat_id IS NOT NULL
                        AND eh.seat_id = esi.seat_id
                  )
                RETURNING esi.id
            )
            SELECT COUNT(*)::bigint
            FROM released_inventory
            "#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(released as usize)
    }
}
