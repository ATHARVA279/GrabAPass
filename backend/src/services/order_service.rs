use sqlx::PgPool;
use uuid::Uuid;
use axum::http::StatusCode;
use crate::db::models::{Order, CheckoutRequest};
use crate::repositories::order_repository::OrderRepository;

pub struct OrderService;

impl OrderService {
    /// Simulates a checkout flow by validating current seat holds
    /// and executing the database transaction to convert them to orders
    /// and generate tickets.
    pub async fn checkout(
        pool: &PgPool,
        event_id: Uuid,
        user_id: Uuid,
        req: CheckoutRequest,
        jwt_secret: &str,
    ) -> Result<Order, (StatusCode, String)> {
        if req.seat_ids.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "No seats provided for checkout.".to_string()));
        }

        let mut tx = pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        // Inside this transaction, the repository verifies holds are active,
        // creates the orders and items, generates tickets, and updates the inventory to 'Sold'.
        let order = OrderRepository::create_checkout_transaction(&mut tx, event_id, user_id, &req.seat_ids, jwt_secret).await?;

        tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(order)
    }

    pub async fn get_user_orders(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Order>, (StatusCode, String)> {
        OrderRepository::list_user_orders(pool, user_id).await
    }
}
