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
        let result = sqlx::query!(
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
            event_id,
            seat_id
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if result.is_none() {
            return Err((StatusCode::BAD_REQUEST, "One or more seats are no longer available.".to_string()));
        }

        // 2. Insert or Update the hold
        let hold = sqlx::query_as!(
            SeatHold,
            r#"
            INSERT INTO seat_holds (event_id, seat_id, user_id, expires_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (event_id, seat_id)
            DO UPDATE SET user_id = EXCLUDED.user_id, expires_at = EXCLUDED.expires_at, created_at = NOW()
            RETURNING id, event_id, seat_id, user_id, created_at, expires_at
            "#,
            event_id,
            seat_id,
            user_id,
            expires_at
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(hold)
    }

    pub async fn fetch_expired_holds(
        pool: &PgPool,
    ) -> Result<Vec<SeatHold>, (StatusCode, String)> {
        let now = chrono::Utc::now();
        let holds = sqlx::query_as!(
            SeatHold,
            r#"
            SELECT id, event_id, seat_id, user_id, created_at, expires_at
            FROM seat_holds
            WHERE expires_at <= $1
            "#,
            now
        )
        .fetch_all(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(holds)
    }

    pub async fn remove_hold_and_release_seat_transaction(
        tx: &mut Transaction<'_, Postgres>,
        hold_id: Uuid,
        event_id: Uuid,
        seat_id: Uuid,
    ) -> Result<(), (StatusCode, String)> {
        // Delete the hold
        sqlx::query!(
            r#"
            DELETE FROM seat_holds WHERE id = $1
            "#,
            hold_id
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Return seat back to Available
        sqlx::query!(
            r#"
            UPDATE event_seat_inventory
            SET status = 'Available'::seat_status
            WHERE event_id = $1 AND seat_id = $2 AND status = 'Held'::seat_status
            "#,
            event_id,
            seat_id
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }
}
