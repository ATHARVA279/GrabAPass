use std::collections::VecDeque;
use std::time::{Duration, Instant};

use axum::http::{HeaderMap, StatusCode};

use crate::SharedRateLimiter;

pub struct RateLimitService;

impl RateLimitService {
    pub async fn check_limit(
        limiter: &SharedRateLimiter,
        scope: &str,
        actor: &str,
        max_requests: usize,
        window: Duration,
    ) -> Result<(), (StatusCode, String)> {
        let now = Instant::now();
        let cutoff = now - window;
        let key = format!("{scope}:{actor}");

        let mut store = limiter.lock().await;
        let bucket = store.entry(key).or_insert_with(VecDeque::new);

        while matches!(bucket.front(), Some(timestamp) if *timestamp < cutoff) {
            bucket.pop_front();
        }

        if bucket.len() >= max_requests {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                format!("Too many requests for {scope}. Please wait and try again."),
            ));
        }

        bucket.push_back(now);
        Ok(())
    }

    pub fn actor_from_headers(headers: &HeaderMap) -> String {
        headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .or_else(|| {
                headers
                    .get("x-real-ip")
                    .and_then(|value| value.to_str().ok())
            })
            .unwrap_or("local")
            .trim()
            .to_string()
    }
}
