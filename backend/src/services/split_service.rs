use axum::http::StatusCode;
use chrono::{Duration, Utc};
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

use crate::db::models::{InitializeSplitRequest, SplitSession, SplitStatus, TicketDetail};
use crate::repositories::order_repository::OrderRepository;
use crate::repositories::split_repository::SplitRepository;
use crate::repositories::ticket_repository::TicketRepository;
use crate::services::payment_service::PaymentService;

pub struct SplitService;

#[derive(Debug, FromRow)]
struct AllocatableOrderItem {
    id: Uuid,
    seat_id: Option<Uuid>,
    ticket_tier_id: Option<Uuid>,
    price: f64,
}

#[derive(Debug)]
struct SplitSharePlan {
    is_host_share: bool,
    guest_name: Option<String>,
    guest_email: Option<String>,
    item_ids: Vec<Uuid>,
    subtotal_amount: f64,
}

impl SplitService {
    pub async fn initialize_split(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
        req: InitializeSplitRequest,
    ) -> Result<SplitSession, (StatusCode, String)> {
        // 1. Get the order and verify the user
        let order = crate::repositories::order_repository::OrderRepository::get_order_by_id_for_user(pool, order_id, user_id).await?;

        if order.status != "Pending" {
            return Err((StatusCode::BAD_REQUEST, "Only pending orders can be split.".to_string()));
        }

        let mut tx = pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // 2. Extend the hold to 30 mins
        let new_expiration = Utc::now() + Duration::minutes(30);
        sqlx::query(
            "UPDATE seat_holds SET expires_at = $1 WHERE event_id = $2 AND user_id = $3"
        )
        .bind(new_expiration)
        .bind(order.event_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to extend holds: {}", e)))?;

        // 3. Create split session
        let mut session = SplitRepository::create_split_session_tx(
            &mut tx,
            order_id,
            order.total_amount,
            req.split_type.clone(),
            new_expiration
        ).await?;

        let order_items = sqlx::query_as::<_, AllocatableOrderItem>(
            r#"
            SELECT id, seat_id, ticket_tier_id, price
            FROM order_items
            WHERE order_id = $1
            ORDER BY price DESC, id ASC
            FOR UPDATE
            "#,
        )
        .bind(order_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load order items for split allocation: {}", e)))?;

        let share_plans = match req.split_type {
            crate::db::models::SplitType::Even => {
                let num_shares = req.num_shares.unwrap_or(2);
                if num_shares < 2 {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "At least two shares are required for split checkout.".to_string(),
                    ));
                }

                Self::build_even_share_plans(&order_items, num_shares, order.subtotal_amount)?
            }
            crate::db::models::SplitType::Custom => {
                let shares = req.custom_shares.ok_or((
                    StatusCode::BAD_REQUEST,
                    "Custom shares must be provided.".to_string(),
                ))?;

                Self::build_custom_share_plans(&order_items, shares)?
            }
        };

        let share_amounts = Self::compute_share_amounts(
            &share_plans
                .iter()
                .map(|plan| plan.subtotal_amount)
                .collect::<Vec<_>>(),
            order.total_amount,
        )?;

        let mut shares_created = Vec::with_capacity(share_plans.len());

        for (plan, amount_due) in share_plans.iter().zip(share_amounts.iter()) {
            let share = SplitRepository::create_split_share_tx(
                &mut tx,
                session.id,
                *amount_due,
                plan.is_host_share,
                plan.guest_name.clone(),
                plan.guest_email.clone(),
            )
            .await?;

            for item_id in &plan.item_ids {
                SplitRepository::create_share_item_allocation_tx(&mut tx, share.id, *item_id).await?;
            }

            shares_created.push(share);
        }

        session.shares = Some(shares_created);

        tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(session)
    }

    pub async fn process_share_payment(
        pool: &PgPool,
        gateway_payment_id: &str,
        gateway_order_id: &str,
        jwt_secret: &str,
    ) -> Result<(), (StatusCode, String)> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let share_opt = sqlx::query_as::<_, crate::db::models::SplitShare>(
            r#"
            SELECT id, split_session_id, amount_due::float8, status, guest_name, guest_email, payment_token,
                   is_host_share,
                   gateway_order_id, gateway_payment_id, paid_at, claimed_by_user_id, claimed_ticket_id,
                   claimed_at, created_at, pending_manual_refund
            FROM split_shares
            WHERE gateway_order_id = $1
            FOR UPDATE
            "#,
        )
        .bind(gateway_order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let share = match share_opt {
            None => {
                tx.rollback().await.ok();
                return Ok(());
            }
            Some(s) => s,
        };

        if share.status != SplitStatus::Completed {
            sqlx::query(
                r#"
                UPDATE split_shares
                SET status = 'Completed'::split_status,
                    gateway_payment_id = $2,
                    paid_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(share.id)
            .bind(gateway_payment_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let session_row = sqlx::query(
            r#"
            SELECT id, order_id, status
            FROM split_sessions
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(share.split_session_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let session_status: SplitStatus = session_row
            .try_get("status")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if session_status == SplitStatus::Completed {
            tx.commit()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Ok(());
        }

        let share_statuses = sqlx::query_as::<_, (SplitStatus,)>(
            r#"
            SELECT status
            FROM split_shares
            WHERE split_session_id = $1
            FOR UPDATE
            "#,
        )
        .bind(share.split_session_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let all_completed = share_statuses
            .iter()
            .all(|(status,)| *status == SplitStatus::Completed);

        if !all_completed {
            tx.commit()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Ok(());
        }

        let order_id: Uuid = session_row
            .try_get("order_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let row = sqlx::query("SELECT id, user_id, event_id FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let user_id: Uuid = row.try_get("user_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let event_id: Uuid = row.try_get("event_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        OrderRepository::finalize_split_order(&mut tx, order_id, event_id, user_id, jwt_secret).await?;

        sqlx::query(
            "UPDATE split_sessions SET status = 'Completed'::split_status WHERE id = $1",
        )
        .bind(share.split_session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tx.commit().await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn claim_share_ticket(
        pool: &PgPool,
        token: Uuid,
        claimant_user_id: Uuid,
        claimant_email: &str,
        jwt_secret: &str,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let share_row = sqlx::query(
            r#"
            SELECT
                s.id,
                s.split_session_id,
                s.status,
                s.is_host_share,
                s.guest_email,
                s.claimed_by_user_id,
                s.claimed_ticket_id,
                ss.status AS session_status,
                ss.order_id,
                o.user_id AS host_user_id,
                o.event_id
            FROM split_shares s
            JOIN split_sessions ss ON ss.id = s.split_session_id
            JOIN orders o ON o.id = ss.order_id
            WHERE s.payment_token = $1
            FOR UPDATE
            "#,
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Token invalid or expired.".to_string()))?;

        let share_id: Uuid = share_row
            .try_get("id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let share_status: SplitStatus = share_row
            .try_get("status")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let session_status: SplitStatus = share_row
            .try_get("session_status")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let guest_email: Option<String> = share_row
            .try_get("guest_email")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let claimed_by_user_id: Option<Uuid> = share_row
            .try_get("claimed_by_user_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let claimed_ticket_id: Option<Uuid> = share_row
            .try_get("claimed_ticket_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let order_id: Uuid = share_row
            .try_get("order_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let host_user_id: Uuid = share_row
            .try_get("host_user_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let event_id: Uuid = share_row
            .try_get("event_id")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let is_host_share: bool = share_row
            .try_get("is_host_share")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if session_status != SplitStatus::Completed {
            tx.rollback().await.ok();
            return Err((
                StatusCode::CONFLICT,
                "Tickets can only be claimed after every split payment has completed.".to_string(),
            ));
        }

        if share_status != SplitStatus::Completed {
            tx.rollback().await.ok();
            return Err((
                StatusCode::CONFLICT,
                "This split share must be paid before its tickets can be claimed.".to_string(),
            ));
        }

        if is_host_share {
            tx.rollback().await.ok();
            return Err((
                StatusCode::CONFLICT,
                "The host share stays in the booking owner's account and cannot be claimed.".to_string(),
            ));
        }

        if claimant_user_id == host_user_id {
            tx.rollback().await.ok();
            return Err((
                StatusCode::FORBIDDEN,
                "The booking owner cannot claim a guest allocation.".to_string(),
            ));
        }

        if let Some(existing_owner) = claimed_by_user_id {
            if existing_owner == claimant_user_id {
                let ticket_id = claimed_ticket_id.ok_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "This share is already claimed, but the ticket reference is missing.".to_string(),
                ))?;
                let detail = TicketRepository::get_ticket_detail_in_tx(&mut tx, ticket_id).await?;
                tx.commit()
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                return Ok(detail);
            }

            tx.rollback().await.ok();
            return Err((
                StatusCode::CONFLICT,
                "This split share has already been claimed by another account.".to_string(),
            ));
        }

        if let Some(email) = guest_email {
            if email.trim().to_lowercase() != claimant_email.trim().to_lowercase() {
                tx.rollback().await.ok();
                return Err((
                    StatusCode::FORBIDDEN,
                    "This split share can only be claimed by the invited email address.".to_string(),
                ));
            }
        }

        let allocated_items = sqlx::query_as::<_, AllocatableOrderItem>(
            r#"
            SELECT oi.id, oi.seat_id, oi.ticket_tier_id, oi.price
            FROM split_share_order_item_allocations ssia
            JOIN order_items oi ON oi.id = ssia.order_item_id
            WHERE ssia.split_share_id = $1
            ORDER BY oi.price DESC, oi.id ASC
            FOR UPDATE OF oi
            "#,
        )
        .bind(share_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if allocated_items.is_empty() {
            tx.rollback().await.ok();
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "No ticket allocation was found for this split share.".to_string(),
            ));
        }

        let host_ticket_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT id
            FROM tickets
            WHERE order_id = $1 AND user_id = $2
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(order_id)
        .bind(host_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "The booking owner's ticket could not be found for claim processing.".to_string(),
        ))?;

        let mut seat_ids = Vec::new();
        let mut tier_quantities = std::collections::BTreeMap::<Uuid, i32>::new();

        for item in &allocated_items {
            match (item.seat_id, item.ticket_tier_id) {
                (Some(seat_id), None) => seat_ids.push(seat_id),
                (None, Some(ticket_tier_id)) => {
                    *tier_quantities.entry(ticket_tier_id).or_insert(0) += 1;
                }
                _ => {
                    tx.rollback().await.ok();
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "A split allocation item is malformed.".to_string(),
                    ));
                }
            }
        }

        let guest_ticket = TicketRepository::create_ticket_in_tx(
            &mut tx,
            order_id,
            event_id,
            &seat_ids,
            &tier_quantities
                .iter()
                .map(|(ticket_tier_id, quantity)| (*ticket_tier_id, *quantity))
                .collect::<Vec<_>>(),
            claimant_user_id,
            jwt_secret,
        )
        .await?;

        if !seat_ids.is_empty() {
            sqlx::query(
                r#"
                DELETE FROM ticket_seats
                WHERE ticket_id = $1
                  AND seat_id = ANY($2)
                "#,
            )
            .bind(host_ticket_id)
            .bind(&seat_ids)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        for (ticket_tier_id, quantity) in tier_quantities {
            let remaining_quantity = sqlx::query_scalar::<_, i32>(
                r#"
                SELECT quantity
                FROM ticket_tiers
                WHERE ticket_id = $1 AND ticket_tier_id = $2
                FOR UPDATE
                "#,
            )
            .bind(host_ticket_id)
            .bind(ticket_tier_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "The host ticket is missing a tier allocation required for claim.".to_string(),
            ))?;

            if remaining_quantity < quantity {
                tx.rollback().await.ok();
                return Err((
                    StatusCode::CONFLICT,
                    "The host ticket no longer contains the expected ticket quantity for this claim.".to_string(),
                ));
            }

            if remaining_quantity == quantity {
                sqlx::query(
                    "DELETE FROM ticket_tiers WHERE ticket_id = $1 AND ticket_tier_id = $2",
                )
                .bind(host_ticket_id)
                .bind(ticket_tier_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            } else {
                sqlx::query(
                    r#"
                    UPDATE ticket_tiers
                    SET quantity = quantity - $3
                    WHERE ticket_id = $1 AND ticket_tier_id = $2
                    "#,
                )
                .bind(host_ticket_id)
                .bind(ticket_tier_id)
                .bind(quantity)
                .execute(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }

        sqlx::query(
            r#"
            UPDATE split_shares
            SET claimed_by_user_id = $2,
                claimed_ticket_id = $3,
                claimed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(share_id)
        .bind(claimant_user_id)
        .bind(guest_ticket.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let detail = TicketRepository::get_ticket_detail_in_tx(&mut tx, guest_ticket.id).await?;

        tx.commit()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(detail)
    }

    pub async fn expire_split_sessions(
        pool: &PgPool,
        config: &crate::RazorpayConfig,
    ) -> Result<(), (StatusCode, String)> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let expired_sessions =
            SplitRepository::find_expired_pending_sessions(&mut tx).await?;

        for (session_id, order_id) in expired_sessions {
            // a. Mark session expired
            if let Err(e) = sqlx::query(
                "UPDATE split_sessions SET status = 'Expired'::split_status WHERE id = $1",
            )
            .bind(session_id)
            .execute(&mut *tx)
            .await
            {
                tracing::error!("Failed to expire session {session_id}: {e}");
                continue;
            }

            // b. Mark order expired
            if let Err(e) = sqlx::query(
                "UPDATE orders SET status = 'Expired' WHERE id = $1",
            )
            .bind(order_id)
            .execute(&mut *tx)
            .await
            {
                tracing::error!("Failed to expire order {order_id}: {e}");
                continue;
            }

            // c. Release seats
            if let Err(e) = sqlx::query(
                r#"
                UPDATE event_seat_inventory
                SET status = 'Available'::seat_status
                WHERE event_id = (SELECT event_id FROM orders WHERE id = $1)
                  AND seat_id IN (
                      SELECT seat_id FROM order_items
                      WHERE order_id = $1 AND seat_id IS NOT NULL
                  )
                "#,
            )
            .bind(order_id)
            .execute(&mut *tx)
            .await
            {
                tracing::error!("Failed to release seats for order {order_id}: {e}");
                continue;
            }

            // d. Delete seat_holds
            if let Err(e) = sqlx::query(
                r#"
                DELETE FROM seat_holds
                WHERE event_id = (SELECT event_id FROM orders WHERE id = $1)
                  AND user_id = (SELECT user_id FROM orders WHERE id = $1)
                "#,
            )
            .bind(order_id)
            .execute(&mut *tx)
            .await
            {
                tracing::error!("Failed to delete seat holds for order {order_id}: {e}");
                continue;
            }

            // e. Fetch shares for the session (uses pool, not tx)
            let shares = match SplitRepository::get_shares_for_session(pool, session_id).await {
                Ok(s) => s,
                Err((_, e)) => {
                    tracing::error!("Failed to fetch shares for session {session_id}: {e}");
                    continue;
                }
            };

            // f. Refund completed shares
            for share in &shares {
                if share.status != SplitStatus::Completed {
                    continue;
                }
                let Some(ref gateway_payment_id) = share.gateway_payment_id else {
                    continue;
                };
                let amount_paise = (share.amount_due * 100.0).round() as i64;

                match PaymentService::refund_payment(config, gateway_payment_id, amount_paise).await {
                    Ok(()) => {
                        if let Err(e) = sqlx::query(
                            "UPDATE split_shares SET status = 'Refunded'::split_status WHERE id = $1",
                        )
                        .bind(share.id)
                        .execute(&mut *tx)
                        .await
                        {
                            tracing::error!("Failed to mark share {} as Refunded: {e}", share.id);
                        }
                    }
                    Err((_, e)) => {
                        tracing::error!(
                            "Refund failed for share {} (payment {}): {e}",
                            share.id,
                            gateway_payment_id
                        );
                        if let Err(db_err) = sqlx::query(
                            r#"
                            UPDATE split_shares
                            SET status = 'Refunded'::split_status, pending_manual_refund = true
                            WHERE id = $1
                            "#,
                        )
                        .bind(share.id)
                        .execute(&mut *tx)
                        .await
                        {
                            tracing::error!(
                                "Failed to mark share {} for manual refund: {db_err}",
                                share.id
                            );
                        }
                    }
                }
            }

            // g. Mark remaining Pending shares as Expired
            if let Err(e) = sqlx::query(
                r#"
                UPDATE split_shares
                SET status = 'Expired'::split_status
                WHERE split_session_id = $1 AND status = 'Pending'::split_status
                "#,
            )
            .bind(session_id)
            .execute(&mut *tx)
            .await
            {
                tracing::error!("Failed to expire pending shares for session {session_id}: {e}");
            }
        }

        tx.commit()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn initialize_share_checkout(
        pool: &PgPool,
        token: Uuid,
        config: &crate::RazorpayConfig,
    ) -> Result<crate::db::models::SplitCheckoutInitialization, (StatusCode, String)> {
        let detail = SplitRepository::get_split_share_public_details(pool, token).await?;
        let existing_share = SplitRepository::get_share_by_payment_token(pool, token).await?;

        if detail.status != crate::db::models::SplitStatus::Pending {
            return Err((StatusCode::BAD_REQUEST, "Share is already paid or expired.".to_string()));
        }

        if detail.session_expires_at < chrono::Utc::now() {
            return Err((StatusCode::BAD_REQUEST, "The session for this split has expired.".to_string()));
        }

        let amount_paise = (detail.amount_due * 100.0).round() as i64;

        if let Some(existing_gateway_order_id) = existing_share.gateway_order_id.clone() {
            return Ok(crate::db::models::SplitCheckoutInitialization {
                share_id: existing_share.id,
                split_session_id: existing_share.split_session_id,
                gateway: "Razorpay".to_string(),
                gateway_key_id: config.key_id.clone(),
                gateway_order_id: existing_gateway_order_id,
                amount: amount_paise,
                currency: "INR".to_string(),
                customer_name: if detail.is_host_share {
                    detail.host_name.clone()
                } else {
                    detail.guest_name.unwrap_or(format!("Guest of {}", detail.host_name))
                },
                customer_email: existing_share.guest_email.unwrap_or_default(),
            });
        }

        let rzp_order = crate::services::payment_service::PaymentService::create_order(
            config,
            crate::services::payment_service::RazorpayCreateOrderRequest {
                amount: amount_paise,
                currency: "INR".to_string(),
                receipt: format!("sh_{}", &detail.id.to_string()[..32]),
                notes: serde_json::json!({
                    "split_share_id": detail.id.to_string(),
                }),
            },
        )
        .await?;

        let mut tx = pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let share = SplitRepository::attach_gateway_to_share_tx(&mut tx, token, &rzp_order.id).await?;

        tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(crate::db::models::SplitCheckoutInitialization {
            share_id: share.id,
            split_session_id: share.split_session_id,
            gateway: "Razorpay".to_string(),
            gateway_key_id: config.key_id.clone(),
            gateway_order_id: rzp_order.id.clone(),
            amount: rzp_order.amount,
            currency: rzp_order.currency,
            customer_name: if detail.is_host_share {
                detail.host_name
            } else {
                detail.guest_name.unwrap_or(format!("Guest of {}", detail.host_name))
            },
            customer_email: share.guest_email.unwrap_or_default(),
        })
    }

    fn build_even_share_plans(
        order_items: &[AllocatableOrderItem],
        num_shares: i32,
        order_subtotal: f64,
    ) -> Result<Vec<SplitSharePlan>, (StatusCode, String)> {
        if order_items.len() < num_shares as usize {
            return Err((
                StatusCode::BAD_REQUEST,
                "Each split share must contain at least one ticket item.".to_string(),
            ));
        }

        let mut buckets = (0..num_shares)
            .map(|_| {
                (
                    0.0_f64,
                    Vec::<Uuid>::new(),
                )
            })
            .collect::<Vec<_>>();

        let target_subtotal = order_subtotal / f64::from(num_shares);

        let mut items_iter = order_items.iter();
        for bucket in &mut buckets {
            let Some(item) = items_iter.next() else {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Split allocation failed because ticket items ran out unexpectedly.".to_string(),
                ));
            };

            bucket.0 += item.price;
            bucket.1.push(item.id);
        }

        for item in items_iter {
            let bucket_index = buckets
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    let left_distance = (left.0 - target_subtotal).abs();
                    let right_distance = (right.0 - target_subtotal).abs();

                    left_distance
                        .partial_cmp(&right_distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal))
                })
                .map(|(index, _)| index)
                .ok_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Split allocation failed because no share bucket was available.".to_string(),
                ))?;

            buckets[bucket_index].0 += item.price;
            buckets[bucket_index].1.push(item.id);
        }

        Ok(buckets
            .into_iter()
            .enumerate()
            .map(|(index, (allocated_subtotal, item_ids))| SplitSharePlan {
                is_host_share: index == 0,
                guest_name: None,
                guest_email: None,
                item_ids,
                subtotal_amount: allocated_subtotal,
            })
            .collect())
    }

    fn build_custom_share_plans(
        order_items: &[AllocatableOrderItem],
        shares: Vec<crate::db::models::CustomShareRequest>,
    ) -> Result<Vec<SplitSharePlan>, (StatusCode, String)> {
        if shares.len() < 2 {
            return Err((
                StatusCode::BAD_REQUEST,
                "At least two shares are required for split checkout.".to_string(),
            ));
        }

        let mut seat_items = std::collections::HashMap::<Uuid, (Uuid, f64)>::new();
        let mut tier_items = std::collections::HashMap::<Uuid, Vec<(Uuid, f64)>>::new();

        for item in order_items {
            match (item.seat_id, item.ticket_tier_id) {
                (Some(seat_id), None) => {
                    seat_items.insert(seat_id, (item.id, item.price));
                }
                (None, Some(ticket_tier_id)) => {
                    tier_items
                        .entry(ticket_tier_id)
                        .or_default()
                        .push((item.id, item.price));
                }
                _ => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Order item is missing its purchase target.".to_string(),
                    ));
                }
            }
        }

        for items in tier_items.values_mut() {
            items.sort_by(|left, right| left.0.cmp(&right.0));
        }

        let mut plans = Vec::with_capacity(shares.len());

        for (index, share) in shares.into_iter().enumerate() {
            let is_host_share = index == 0;
            let mut item_ids = Vec::new();
            let mut subtotal_amount = 0.0;

            for seat_id in share.seat_ids {
                let (item_id, price) = seat_items.remove(&seat_id).ok_or((
                    StatusCode::BAD_REQUEST,
                    "Each reserved seat must be assigned exactly once across split shares.".to_string(),
                ))?;
                item_ids.push(item_id);
                subtotal_amount += price;
            }

            if let Some(ticket_tiers) = share.ticket_tiers {
                for tier in ticket_tiers {
                    if tier.quantity <= 0 {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "Assigned ticket tier quantity must be at least 1.".to_string(),
                        ));
                    }

                    let available_items = tier_items.get_mut(&tier.ticket_tier_id).ok_or((
                        StatusCode::BAD_REQUEST,
                        "Each general admission ticket must be assigned exactly once across split shares.".to_string(),
                    ))?;

                    if available_items.len() < tier.quantity as usize {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "A split share is assigning more general admission tickets than were selected.".to_string(),
                        ));
                    }

                    for _ in 0..tier.quantity {
                        let (item_id, price) = available_items.remove(0);
                        item_ids.push(item_id);
                        subtotal_amount += price;
                    }
                }
            }

            if item_ids.is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Every split share must be assigned at least one ticket.".to_string(),
                ));
            }

            if !is_host_share && share.guest_email.as_deref().map(str::trim).unwrap_or("").is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Each guest share needs an email so the right person can claim their ticket.".to_string(),
                ));
            }

            plans.push(SplitSharePlan {
                is_host_share,
                guest_name: if is_host_share { None } else { share.guest_name.filter(|value| !value.trim().is_empty()) },
                guest_email: if is_host_share {
                    None
                } else {
                    share.guest_email.map(|value| value.trim().to_lowercase())
                },
                item_ids,
                subtotal_amount,
            });
        }

        if !seat_items.is_empty() || tier_items.values().any(|items| !items.is_empty()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Assign every selected ticket to exactly one share before generating split links.".to_string(),
            ));
        }

        Ok(plans)
    }

    fn compute_share_amounts(
        subtotals: &[f64],
        order_total: f64,
    ) -> Result<Vec<f64>, (StatusCode, String)> {
        if subtotals.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "At least one split share is required.".to_string(),
            ));
        }

        let subtotal_paise: Vec<i64> = subtotals
            .iter()
            .map(|value| (value * 100.0).round() as i64)
            .collect();
        let subtotal_total_paise: i64 = subtotal_paise.iter().sum();
        let total_paise = (order_total * 100.0).round() as i64;

        if subtotal_total_paise <= 0 || total_paise < subtotal_total_paise {
            return Err((
                StatusCode::BAD_REQUEST,
                "Split shares could not be priced correctly for this order.".to_string(),
            ));
        }

        let fee_total_paise = total_paise - subtotal_total_paise;
        let mut fee_allocations = vec![0_i64; subtotal_paise.len()];
        let mut remainders = Vec::with_capacity(subtotal_paise.len());

        for (index, subtotal) in subtotal_paise.iter().enumerate() {
            let numerator = i128::from(*subtotal) * i128::from(fee_total_paise);
            fee_allocations[index] = (numerator / i128::from(subtotal_total_paise)) as i64;
            remainders.push((index, numerator % i128::from(subtotal_total_paise)));
        }

        let allocated_fee: i64 = fee_allocations.iter().sum();
        let mut fee_remainder = fee_total_paise - allocated_fee;
        remainders.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

        for (index, _) in remainders {
            if fee_remainder <= 0 {
                break;
            }

            fee_allocations[index] += 1;
            fee_remainder -= 1;
        }

        Ok(subtotal_paise
            .iter()
            .zip(fee_allocations.iter())
            .map(|(subtotal, fee)| (*subtotal + *fee) as f64 / 100.0)
            .collect())
    }
}
