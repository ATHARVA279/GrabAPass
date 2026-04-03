use axum::http::StatusCode;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        CheckoutFailureRequest, Claims, InitializeCheckoutRequest, InitializeCheckoutResponse,
        Order, RazorpayWebhookPayload, VerifyCheckoutRequest,
    },
    repositories::order_repository::OrderRepository,
    services::payment_service::{PaymentService, RazorpayCreateOrderRequest},
    services::split_service::SplitService,
    services::suspicious_activity_service::SuspiciousActivityService,
};

pub struct OrderService;

impl OrderService {
    pub async fn initialize_checkout(
        state: &AppState,
        event_id: Uuid,
        claims: &Claims,
        req: InitializeCheckoutRequest,
    ) -> Result<InitializeCheckoutResponse, (StatusCode, String)> {
        if req.hold_ids.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "No active ticket holds were provided for checkout.".to_string(),
            ));
        }

        let razorpay = state.razorpay.as_ref().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Payment gateway is not configured.".to_string(),
        ))?;

        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let held_items =
            OrderRepository::get_active_held_items(&mut tx, event_id, claims.sub, &req.hold_ids)
                .await?;

        let subtotal_amount = round_currency(held_items.iter().map(|item| item.price).sum());
        let fee_amount = round_currency(subtotal_amount * 0.02);
        let total_amount = round_currency(subtotal_amount + fee_amount);

        let order = OrderRepository::create_pending_order(
            &mut tx,
            event_id,
            claims.sub,
            subtotal_amount,
            fee_amount,
            total_amount,
            "INR",
        )
        .await?;

        OrderRepository::create_order_items(&mut tx, order.id, &held_items).await?;

        let hold_expires_at = Utc::now() + Duration::minutes(10);
        OrderRepository::extend_hold_expiry(
            &mut tx,
            event_id,
            claims.sub,
            &req.hold_ids,
            hold_expires_at,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let receipt = format!("gpass_{}", order.id.simple());
        let gateway_order = match PaymentService::create_order(
            razorpay,
            RazorpayCreateOrderRequest {
                amount: amount_to_subunits(total_amount),
                currency: "INR".to_string(),
                receipt: receipt.clone(),
                notes: serde_json::json!({
                    "local_order_id": order.id.to_string(),
                    "event_id": event_id.to_string(),
                    "customer_id": claims.sub.to_string(),
                }),
            },
        )
        .await
        {
            Ok(order_response) => order_response,
            Err(error) => {
                let _ = OrderRepository::mark_order_failed(
                    &state.pool,
                    order.id,
                    claims.sub,
                    Some("Unable to create gateway order."),
                    None,
                )
                .await;
                return Err(error);
            }
        };

        let order = OrderRepository::attach_gateway_order(
            &state.pool,
            order.id,
            "Razorpay",
            &gateway_order.id,
            &receipt,
        )
        .await?;

        Ok(InitializeCheckoutResponse {
            order,
            gateway: "Razorpay".to_string(),
            gateway_key_id: razorpay.key_id.clone(),
            gateway_order_id: gateway_order.id,
            amount: gateway_order.amount,
            currency: gateway_order.currency,
            description: format!("GrabAPass booking for event {}", event_id),
            customer_name: claims.name.clone(),
            customer_email: claims.email.clone(),
            hold_expires_at,
        })
    }

    pub async fn verify_checkout(
        state: &AppState,
        event_id: Uuid,
        user_id: Uuid,
        req: VerifyCheckoutRequest,
    ) -> Result<Order, (StatusCode, String)> {
        let razorpay = state.razorpay.as_ref().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Payment gateway is not configured.".to_string(),
        ))?;

        let existing_order =
            OrderRepository::get_order_by_id_for_user(&state.pool, req.order_id, user_id).await?;

        if existing_order.event_id != event_id {
            return Err((
                StatusCode::BAD_REQUEST,
                "Order does not belong to this event.".to_string(),
            ));
        }

        let expected_gateway_order_id = existing_order.gateway_order_id.clone().ok_or((
            StatusCode::BAD_REQUEST,
            "Order is missing the gateway order id.".to_string(),
        ))?;

        if expected_gateway_order_id != req.razorpay_order_id {
            return Err((
                StatusCode::BAD_REQUEST,
                "Gateway order id mismatch.".to_string(),
            ));
        }

        PaymentService::verify_signature(
            &razorpay.key_secret,
            &req.razorpay_order_id,
            &req.razorpay_payment_id,
            &req.razorpay_signature,
        )?;

        Self::reconcile_paid_order(
            state,
            existing_order,
            &req.razorpay_payment_id,
            Some(&req.razorpay_signature),
        )
        .await
    }

    pub async fn record_checkout_failure(
        state: &AppState,
        event_id: Uuid,
        user_id: Uuid,
        req: CheckoutFailureRequest,
    ) -> Result<(), (StatusCode, String)> {
        let order =
            OrderRepository::get_order_by_id_for_user(&state.pool, req.order_id, user_id).await?;
        if order.event_id != event_id {
            return Err((
                StatusCode::BAD_REQUEST,
                "Order does not belong to this event.".to_string(),
            ));
        }

        if let Some(gateway_order_id) = req.razorpay_order_id.as_deref() {
            if order.gateway_order_id.as_deref() != Some(gateway_order_id) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Gateway order id mismatch.".to_string(),
                ));
            }
        }

        OrderRepository::mark_order_failed(
            &state.pool,
            req.order_id,
            user_id,
            req.reason.as_deref(),
            req.razorpay_payment_id.as_deref(),
        )
        .await?;

        SuspiciousActivityService::record_payment_failure_if_suspicious(
            &state.pool,
            event_id,
            user_id,
            req.order_id,
            req.reason.as_deref(),
        )
        .await?;

        Ok(())
    }

    pub async fn get_user_orders(
        state: &AppState,
        user_id: Uuid,
    ) -> Result<Vec<Order>, (StatusCode, String)> {
        OrderRepository::list_user_orders(&state.pool, user_id).await
    }

    pub async fn handle_razorpay_webhook(
        state: &AppState,
        signature: &str,
        provider_event_id: &str,
        body: &[u8],
    ) -> Result<(), (StatusCode, String)> {
        let razorpay = state.razorpay.as_ref().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Payment gateway is not configured.".to_string(),
        ))?;

        let webhook_secret = razorpay.webhook_secret.as_deref().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Razorpay webhook secret is not configured.".to_string(),
        ))?;

        PaymentService::verify_webhook_signature(webhook_secret, body, signature)?;

        let payload: RazorpayWebhookPayload = serde_json::from_slice(body).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid Razorpay webhook payload: {e}"),
            )
        })?;

        let payload_json: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid webhook JSON: {e}"),
            )
        })?;

        let is_new_event = OrderRepository::record_webhook_event(
            &state.pool,
            provider_event_id,
            &payload.event,
            &payload_json,
        )
        .await?;

        if !is_new_event {
            return Ok(());
        }

        match payload.event.as_str() {
            "payment.captured" | "payment.authorized" => {
                let payment = payload
                    .payload
                    .payment
                    .ok_or((
                        StatusCode::BAD_REQUEST,
                        "Webhook missing payment payload.".to_string(),
                    ))?
                    .entity;
                let gateway_order_id = payment.order_id.ok_or((
                    StatusCode::BAD_REQUEST,
                    "Webhook payment missing order id.".to_string(),
                ))?;

                // Try to find a regular order first
                match OrderRepository::get_order_by_gateway_order_id(
                    &state.pool,
                    &gateway_order_id,
                )
                .await
                {
                    Ok(order) => {
                        // Regular order — reconcile as normal
                        Self::reconcile_paid_order(state, order, &payment.id, None).await?;
                    }
                    Err((StatusCode::NOT_FOUND, _)) => {
                        // Not a regular order — try to handle as a split share payment
                        SplitService::process_share_payment(
                            &state.pool,
                            &payment.id,
                            &gateway_order_id,
                            &state.jwt_secret,
                        )
                        .await?;
                    }
                    Err(e) => return Err(e),
                }
            }
            "payment.failed" => {
                let payment = payload
                    .payload
                    .payment
                    .ok_or((
                        StatusCode::BAD_REQUEST,
                        "Webhook missing payment payload.".to_string(),
                    ))?
                    .entity;
                if let Some(gateway_order_id) = payment.order_id.as_deref() {
                    let order = OrderRepository::get_order_by_gateway_order_id(
                        &state.pool,
                        gateway_order_id,
                    )
                    .await?;
                    OrderRepository::mark_order_failed(
                        &state.pool,
                        order.id,
                        order.user_id,
                        Some("Payment failed according to Razorpay webhook."),
                        Some(&payment.id),
                    )
                    .await?;

                    SuspiciousActivityService::record_payment_failure_if_suspicious(
                        &state.pool,
                        order.event_id,
                        order.user_id,
                        order.id,
                        Some("payment.failed webhook"),
                    )
                    .await?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl OrderService {
    async fn reconcile_paid_order(
        state: &AppState,
        existing_order: Order,
        payment_id: &str,
        payment_signature: Option<&str>,
    ) -> Result<Order, (StatusCode, String)> {
        let razorpay = state.razorpay.as_ref().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Payment gateway is not configured.".to_string(),
        ))?;

        let mut payment = PaymentService::fetch_payment(razorpay, payment_id).await?;

        if payment.order_id.as_deref() != existing_order.gateway_order_id.as_deref() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Payment is not linked to the expected Razorpay order.".to_string(),
            ));
        }

        if payment.amount != amount_to_subunits(existing_order.total_amount) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Gateway amount does not match the expected order amount.".to_string(),
            ));
        }

        if payment.currency != existing_order.currency {
            return Err((
                StatusCode::BAD_REQUEST,
                "Gateway currency does not match the expected order currency.".to_string(),
            ));
        }

        if payment.status == "authorized" {
            payment = PaymentService::capture_payment(
                razorpay,
                payment_id,
                payment.amount,
                &payment.currency,
            )
            .await?;
        }

        if payment.status != "captured" && payment.status != "authorized" {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Payment is not capturable. Current status: {}.",
                    payment.status
                ),
            ));
        }

        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let finalized = OrderRepository::finalize_checkout_transaction(
            &mut tx,
            existing_order.id,
            existing_order.event_id,
            existing_order.user_id,
            &state.jwt_secret,
            payment_id,
            payment_signature.unwrap_or("webhook_verified"),
        )
        .await;

        match finalized {
            Ok(order) => {
                tx.commit()
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                Ok(order)
            }
            Err((StatusCode::CONFLICT, message)) => {
                tx.rollback().await.ok();
                OrderRepository::mark_order_manual_review(
                    &state.pool,
                    existing_order.id,
                    existing_order.user_id,
                    payment_id,
                    payment_signature.unwrap_or("webhook_verified"),
                    &message,
                )
                .await?;
                Err((
                    StatusCode::CONFLICT,
                    format!("{message} Order has been marked for manual review."),
                ))
            }
            Err(error) => {
                tx.rollback().await.ok();
                Err(error)
            }
        }
    }
}

fn round_currency(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn amount_to_subunits(value: f64) -> i64 {
    (value * 100.0).round() as i64
}
