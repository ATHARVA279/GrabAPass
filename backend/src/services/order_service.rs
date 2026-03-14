use chrono::{Duration, Utc};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        CheckoutFailureRequest, Claims, InitializeCheckoutRequest, InitializeCheckoutResponse,
        Order, VerifyCheckoutRequest,
    },
    repositories::order_repository::OrderRepository,
    services::payment_service::{PaymentService, RazorpayCreateOrderRequest},
};

pub struct OrderService;

impl OrderService {
    pub async fn initialize_checkout(
        state: &AppState,
        event_id: Uuid,
        claims: &Claims,
        req: InitializeCheckoutRequest,
    ) -> Result<InitializeCheckoutResponse, (StatusCode, String)> {
        if req.seat_ids.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "No seats provided for checkout.".to_string()));
        }

        let razorpay = state
            .razorpay
            .as_ref()
            .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Payment gateway is not configured.".to_string()))?;

        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let held_seats =
            OrderRepository::get_active_held_seats(&mut tx, event_id, claims.sub, &req.seat_ids).await?;

        let subtotal_amount = round_currency(held_seats.iter().map(|seat| seat.price).sum());
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

        OrderRepository::create_order_items(&mut tx, order.id, &held_seats).await?;

        let hold_expires_at = Utc::now() + Duration::minutes(10);
        OrderRepository::extend_hold_expiry(
            &mut tx,
            event_id,
            claims.sub,
            &req.seat_ids,
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
        let razorpay = state
            .razorpay
            .as_ref()
            .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Payment gateway is not configured.".to_string()))?;

        let existing_order =
            OrderRepository::get_order_by_id_for_user(&state.pool, req.order_id, user_id).await?;

        if existing_order.event_id != event_id {
            return Err((StatusCode::BAD_REQUEST, "Order does not belong to this event.".to_string()));
        }

        let expected_gateway_order_id = existing_order
            .gateway_order_id
            .clone()
            .ok_or((StatusCode::BAD_REQUEST, "Order is missing the gateway order id.".to_string()))?;

        if expected_gateway_order_id != req.razorpay_order_id {
            return Err((StatusCode::BAD_REQUEST, "Gateway order id mismatch.".to_string()));
        }

        PaymentService::verify_signature(
            &razorpay.key_secret,
            &req.razorpay_order_id,
            &req.razorpay_payment_id,
            &req.razorpay_signature,
        )?;

        let mut payment = PaymentService::fetch_payment(razorpay, &req.razorpay_payment_id).await?;

        if payment.order_id.as_deref() != Some(req.razorpay_order_id.as_str()) {
            return Err((StatusCode::BAD_REQUEST, "Payment is not linked to the expected Razorpay order.".to_string()));
        }

        if payment.amount != amount_to_subunits(existing_order.total_amount) {
            return Err((StatusCode::BAD_REQUEST, "Gateway amount does not match the expected order amount.".to_string()));
        }

        if payment.currency != existing_order.currency {
            return Err((StatusCode::BAD_REQUEST, "Gateway currency does not match the expected order currency.".to_string()));
        }

        if payment.status == "authorized" {
            payment = PaymentService::capture_payment(
                razorpay,
                &req.razorpay_payment_id,
                payment.amount,
                &payment.currency,
            )
            .await?;
        }

        if payment.status != "captured" && payment.status != "authorized" {
            return Err((StatusCode::BAD_REQUEST, format!("Payment is not capturable. Current status: {}.", payment.status)));
        }

        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let finalized = OrderRepository::finalize_checkout_transaction(
            &mut tx,
            req.order_id,
            event_id,
            user_id,
            &state.jwt_secret,
            &req.razorpay_payment_id,
            &req.razorpay_signature,
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
                    req.order_id,
                    user_id,
                    &req.razorpay_payment_id,
                    &req.razorpay_signature,
                    &message,
                )
                .await?;
                Err((StatusCode::CONFLICT, format!("{message} Order has been marked for manual review.")))
            }
            Err(error) => {
                tx.rollback().await.ok();
                Err(error)
            }
        }
    }

    pub async fn record_checkout_failure(
        state: &AppState,
        event_id: Uuid,
        user_id: Uuid,
        req: CheckoutFailureRequest,
    ) -> Result<(), (StatusCode, String)> {
        let order = OrderRepository::get_order_by_id_for_user(&state.pool, req.order_id, user_id).await?;
        if order.event_id != event_id {
            return Err((StatusCode::BAD_REQUEST, "Order does not belong to this event.".to_string()));
        }

        if let Some(gateway_order_id) = req.razorpay_order_id.as_deref() {
            if order.gateway_order_id.as_deref() != Some(gateway_order_id) {
                return Err((StatusCode::BAD_REQUEST, "Gateway order id mismatch.".to_string()));
            }
        }

        OrderRepository::mark_order_failed(
            &state.pool,
            req.order_id,
            user_id,
            req.reason.as_deref(),
            req.razorpay_payment_id.as_deref(),
        )
        .await
    }

    pub async fn get_user_orders(
        state: &AppState,
        user_id: Uuid,
    ) -> Result<Vec<Order>, (StatusCode, String)> {
        OrderRepository::list_user_orders(&state.pool, user_id).await
    }
}

fn round_currency(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn amount_to_subunits(value: f64) -> i64 {
    (value * 100.0).round() as i64
}
