use crate::AppState;
use crate::db::models::TicketDetail;
use crate::repositories::{auth_repository, order_repository::OrderRepository, ticket_repository::TicketRepository};
use crate::services::email_service::{CancellationEmailData, EmailService, RefundEmailData};
use crate::services::payment_service::PaymentService;
use axum::http::StatusCode;
use uuid::Uuid;

pub struct TicketService;

impl TicketService {
    pub async fn list_user_tickets(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<Vec<TicketDetail>, (StatusCode, String)> {
        TicketRepository::list_user_tickets(pool, user_id).await
    }

    pub async fn get_ticket(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        ticket_id: Uuid,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        TicketRepository::get_ticket_by_id(pool, ticket_id, user_id).await
    }

    pub async fn cancel_ticket(
        state: &AppState,
        user_id: Uuid,
        ticket_id: Uuid,
    ) -> Result<TicketDetail, (StatusCode, String)> {
        let cancellation = TicketRepository::cancel_ticket(&state.pool, ticket_id, user_id).await?;
        let user = auth_repository::find_user_by_id(&state.pool, user_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "User not found.".to_string()))?;
        let order = OrderRepository::get_order_by_id_for_user(&state.pool, cancellation.ticket.order_id, user_id).await?;

        if let Err((_, error)) = EmailService::send_ticket_cancellation(
            state.email.as_ref(),
            CancellationEmailData {
                user: &user,
                order: &order,
                ticket: &cancellation.ticket,
            },
        )
        .await
        {
            tracing::warn!("Failed to send cancellation email: {error}");
        }

        if cancellation.refund_eligible {
            if let Err((_, error)) = EmailService::send_refund_status(
                state.email.as_ref(),
                RefundEmailData {
                    user: &user,
                    order: &order,
                    ticket: &cancellation.ticket,
                    refund_amount: cancellation.refund.amount,
                    refund_status: "Initiated",
                },
            )
            .await
            {
                tracing::warn!("Failed to send refund initiated email: {error}");
            }

            if let (Some(razorpay), Some(payment_id)) =
                (state.razorpay.as_ref(), cancellation.payment_id.as_deref())
            {
                let refund_amount_paise = (cancellation.refund.amount * 100.0).round() as i64;
                match PaymentService::refund_payment(razorpay, payment_id, refund_amount_paise)
                    .await
                {
                    Ok(_) => {
                        TicketRepository::update_refund_status(
                            &state.pool,
                            cancellation.refund.id,
                            "Processed",
                            None,
                        )
                        .await?;

                        if let Err((_, error)) = EmailService::send_refund_status(
                            state.email.as_ref(),
                            RefundEmailData {
                                user: &user,
                                order: &order,
                                ticket: &cancellation.ticket,
                                refund_amount: cancellation.refund.amount,
                                refund_status: "Completed",
                            },
                        )
                        .await
                        {
                            tracing::warn!("Failed to send refund completed email: {error}");
                        }
                    }
                    Err((_status, message)) => {
                        TicketRepository::update_refund_status(
                            &state.pool,
                            cancellation.refund.id,
                            "Failed",
                            Some(message.as_str()),
                        )
                        .await?;
                    }
                }
            } else {
                TicketRepository::update_refund_status(
                    &state.pool,
                    cancellation.refund.id,
                    "Failed",
                    Some("Refund could not be processed because Razorpay payment details are missing."),
                )
                .await?;
            }
        }

        TicketRepository::get_ticket_by_id(&state.pool, ticket_id, user_id).await
    }
}
