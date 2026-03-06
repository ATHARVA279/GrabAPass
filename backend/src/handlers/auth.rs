use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, db::models::UserRole, repositories::auth_repository, services::auth_service,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub name: String,
    pub phone_number: Option<String>,
    pub organizer_company: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub role: UserRole,
    pub name: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    let password_hash = auth_service::hash_password(&payload.password)?;

    let user = auth_repository::create_user(
        &state.pool,
        &payload.email,
        &password_hash,
        &payload.role,
        &payload.name,
        payload.phone_number.as_deref(),
        payload.organizer_company.as_deref(),
    )
    .await
    .map_err(|error| {
        let message = error.to_string();
        if message.contains("duplicate key") || message.contains("unique") {
            (StatusCode::CONFLICT, "Email already in use".to_string())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    })?;

    let token = auth_service::create_jwt(&user, &state.jwt_secret)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id: user.id,
                email: user.email,
                role: user.role,
                name: user.name,
            },
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, String)> {
    let user = auth_repository::find_user_by_email(&state.pool, &payload.email)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    auth_service::verify_password(&payload.password, &user.password_hash)?;

    let token = auth_service::create_jwt(&user, &state.jwt_secret)?;

    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id: user.id,
                email: user.email,
                role: user.role,
                name: user.name,
            },
        }),
    ))
}
