use axum::http::StatusCode;
use hmac::{Hmac, Mac};
use reqwest::StatusCode as ReqwestStatusCode;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::RazorpayConfig;

type HmacSha256 = Hmac<Sha256>;

const RAZORPAY_BASE_URL: &str = "https://api.razorpay.com/v1";

#[derive(Debug, Serialize)]
pub struct RazorpayCreateOrderRequest {
    pub amount: i64,
    pub currency: String,
    pub receipt: String,
    pub notes: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayOrderResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub receipt: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayPaymentResponse {
    pub id: String,
    pub order_id: Option<String>,
    pub amount: i64,
    pub currency: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayRefundResponse {
    pub id: String,
    pub status: String,
    pub amount: i64,
}

pub struct PaymentService;

impl PaymentService {
    pub async fn create_order(
        config: &RazorpayConfig,
        payload: RazorpayCreateOrderRequest,
    ) -> Result<RazorpayOrderResponse, (StatusCode, String)> {
        let response = config
            .client
            .post(format!("{RAZORPAY_BASE_URL}/orders"))
            .basic_auth(&config.key_id, Some(&config.key_secret))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to reach Razorpay: {e}"),
                )
            })?;

        Self::parse_json_response(response).await
    }

    pub async fn fetch_payment(
        config: &RazorpayConfig,
        payment_id: &str,
    ) -> Result<RazorpayPaymentResponse, (StatusCode, String)> {
        let response = config
            .client
            .get(format!("{RAZORPAY_BASE_URL}/payments/{payment_id}"))
            .basic_auth(&config.key_id, Some(&config.key_secret))
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to fetch payment from Razorpay: {e}"),
                )
            })?;

        Self::parse_json_response(response).await
    }

    pub async fn capture_payment(
        config: &RazorpayConfig,
        payment_id: &str,
        amount: i64,
        currency: &str,
    ) -> Result<RazorpayPaymentResponse, (StatusCode, String)> {
        let response = config
            .client
            .post(format!("{RAZORPAY_BASE_URL}/payments/{payment_id}/capture"))
            .basic_auth(&config.key_id, Some(&config.key_secret))
            .json(&serde_json::json!({
                "amount": amount,
                "currency": currency,
            }))
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to capture Razorpay payment: {e}"),
                )
            })?;

        Self::parse_json_response(response).await
    }

    pub fn verify_signature(
        key_secret: &str,
        order_id: &str,
        payment_id: &str,
        signature: &str,
    ) -> Result<(), (StatusCode, String)> {
        let mut mac = HmacSha256::new_from_slice(key_secret.as_bytes()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to initialize HMAC: {e}"),
            )
        })?;
        mac.update(format!("{order_id}|{payment_id}").as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        if expected == signature {
            Ok(())
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                "Payment signature verification failed.".to_string(),
            ))
        }
    }

    pub fn verify_webhook_signature(
        webhook_secret: &str,
        body: &[u8],
        signature: &str,
    ) -> Result<(), (StatusCode, String)> {
        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to initialize HMAC: {e}"),
            )
        })?;
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());

        if expected == signature {
            Ok(())
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                "Webhook signature verification failed.".to_string(),
            ))
        }
    }

    pub async fn refund_payment(
        config: &RazorpayConfig,
        payment_id: &str,
        amount_paise: i64,
    ) -> Result<RazorpayRefundResponse, (StatusCode, String)> {
        let response = config
            .client
            .post(format!("{RAZORPAY_BASE_URL}/payments/{payment_id}/refund"))
            .basic_auth(&config.key_id, Some(&config.key_secret))
            .json(&serde_json::json!({
                "amount": amount_paise,
                "notes": {},
            }))
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to reach Razorpay for refund: {e}"),
                )
            })?;

        Self::parse_json_response(response).await
    }

    async fn parse_json_response<T: for<'de> Deserialize<'de>>(
        response: reqwest::Response,
    ) -> Result<T, (StatusCode, String)> {
        let status = response.status();
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown gateway error".to_string());
            let mapped_status = if status == ReqwestStatusCode::BAD_REQUEST {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::BAD_GATEWAY
            };
            return Err((mapped_status, format!("Razorpay error: {error_body}")));
        }

        response.json::<T>().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Invalid Razorpay response: {e}"),
            )
        })
    }
}
