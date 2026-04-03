use crate::db::models::{HoldSeatsRequest, SeatHold};
use crate::repositories::hold_repository::HoldRepository;
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

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
        if req.seat_ids.is_empty() && req.ticket_tiers.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "No seats or ticket tiers provided for holding.".to_string(),
            ));
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let mut holds = Vec::new();

        let expires_at = Utc::now() + Duration::minutes(10);

        for seat_id in req.seat_ids {
            let hold = HoldRepository::create_hold_transaction(
                &mut tx, event_id, seat_id, user_id, expires_at,
            )
            .await?;
            holds.push(hold);
        }

        for tier in req.ticket_tiers {
            if tier.quantity <= 0 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Ticket tier quantity must be at least 1.".to_string(),
                ));
            }

            let mut tier_holds = HoldRepository::create_ga_holds_transaction(
                &mut tx,
                event_id,
                tier.ticket_tier_id,
                user_id,
                tier.quantity,
                expires_at,
            )
            .await?;
            holds.append(&mut tier_holds);
        }

        tx.commit()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(holds)
    }

    /// Background task function to release expired holds back to available.
    pub async fn release_expired_holds(pool: &PgPool) -> Result<usize, (StatusCode, String)> {
        HoldRepository::release_expired_holds(pool).await
    }
}
