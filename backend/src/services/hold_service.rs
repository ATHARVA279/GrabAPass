use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Duration, Utc};
use axum::http::StatusCode;
use crate::db::models::{SeatHold, HoldSeatsRequest};
use crate::repositories::hold_repository::HoldRepository;

pub struct HoldService;

impl HoldService {
    /// Attempts to lock a list of seats for a particular user for 10 minutes.
    /// If any single seat fails to lock (i.e. already held/sold) the entire transaction is rolled back.
    pub async fn hold_seats(
        pool: &PgPool,
        event_id: Uuid,
        user_id: Uuid,
        req: HoldSeatsRequest,
    ) -> Result<Vec<SeatHold>, (StatusCode, String)> {
        if req.seat_ids.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "No seats provided for holding.".to_string()));
        }

        let mut tx = pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let mut holds = Vec::new();

        let expires_at = Utc::now() + Duration::minutes(10);

        for seat_id in req.seat_ids {
            let hold = HoldRepository::create_hold_transaction(&mut tx, event_id, seat_id, user_id, expires_at).await?;
            holds.push(hold);
        }

        tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(holds)
    }

    /// Background task function to release expired holds back to available.
    pub async fn release_expired_holds(pool: &PgPool) -> Result<usize, (StatusCode, String)> {
        let expired_holds = HoldRepository::fetch_expired_holds(pool).await?;
        let mut released_count = 0;

        for hold in expired_holds {
            let mut tx = match pool.begin().await {
                Ok(tx) => tx,
                Err(_) => continue, // Log or skip on failure
            };
            // Ignore error inside loop so we don't crash the whole sweeping job
            if let Ok(_) = HoldRepository::remove_hold_and_release_seat_transaction(&mut tx, hold.id, hold.event_id, hold.seat_id).await {
                if let Ok(_) = tx.commit().await {
                    released_count += 1;
                }
            }
        }

        Ok(released_count)
    }
}
