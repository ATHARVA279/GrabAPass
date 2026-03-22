use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use axum::http::StatusCode;
use crate::db::models::SeatHold;

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
            return Err((StatusCode::BAD_REQUEST, "One or more seats are no longer available.".to_string()));
        }

        // 2. Insert or Update the hold
        let hold = sqlx::query_as::<_, SeatHold>(
            r#"
            INSERT INTO seat_holds (event_id, seat_id, user_id, expires_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (event_id, seat_id)
            DO UPDATE SET user_id = EXCLUDED.user_id, expires_at = EXCLUDED.expires_at, created_at = NOW()
            RETURNING id, event_id, seat_id, user_id, created_at, expires_at
            "#,
        )
        .bind(event_id)
        .bind(seat_id)
        .bind(user_id)
        .bind(expires_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(hold)
    }

    pub async fn release_expired_holds(
        pool: &PgPool,
    ) -> Result<usize, (StatusCode, String)> {
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
