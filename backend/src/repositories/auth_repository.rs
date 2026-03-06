use sqlx::PgPool;

use crate::db::models::{User, UserRole};

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    role: &UserRole,
    name: &str,
    phone_number: Option<&str>,
    organizer_company: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, role, name, phone_number, organizer_company)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, email, password_hash, role, name, phone_number, organizer_company, created_at
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .bind(name)
    .bind(phone_number)
    .bind(organizer_company)
    .fetch_one(pool)
    .await
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, password_hash, role, name, phone_number, organizer_company, created_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}
