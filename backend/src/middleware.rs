use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};

use crate::AppState;
use crate::models::{Claims, UserRole};

pub struct RequireAuth(pub Claims);

impl FromRequestParts<AppState> for RequireAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));

        let token = auth_header.ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header".to_string(),
        ))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

        Ok(RequireAuth(token_data.claims))
    }
}

// Extractor to specifically require Organizer role
pub struct RequireOrganizer(pub Claims);

impl FromRequestParts<AppState> for RequireOrganizer {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let require_auth = RequireAuth::from_request_parts(parts, state).await?;

        if require_auth.0.role != UserRole::Organizer {
            return Err((StatusCode::FORBIDDEN, "Requires Organizer role".to_string()));
        }

        Ok(RequireOrganizer(require_auth.0))
    }
}
