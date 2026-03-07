use sqlx::PgPool;
use uuid::Uuid;

use crate::db::models::Event;

pub async fn list_published_events(
    pool: &PgPool,
    category: Option<&str>,
    search: &str,
) -> Result<Vec<Event>, sqlx::Error> {
    // Only apply search filter when the caller actually provided a term.
    let apply_search = !search.is_empty();
    let search_pattern = format!("%{search}%");

    match (category, apply_search) {
        (Some(cat), true) => sqlx::query_as::<_, Event>(
            r#"
            SELECT id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
            FROM events
            WHERE status = 'Published'
              AND category = $1
              AND (title ILIKE $2 OR COALESCE(description, '') ILIKE $2)
            ORDER BY start_time ASC
            "#,
        )
        .bind(cat)
        .bind(&search_pattern)
        .fetch_all(pool)
        .await,

        (Some(cat), false) => sqlx::query_as::<_, Event>(
            r#"
            SELECT id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
            FROM events
            WHERE status = 'Published'
              AND category = $1
            ORDER BY start_time ASC
            "#,
        )
        .bind(cat)
        .fetch_all(pool)
        .await,

        (None, true) => sqlx::query_as::<_, Event>(
            r#"
            SELECT id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
            FROM events
            WHERE status = 'Published'
              AND (title ILIKE $1 OR COALESCE(description, '') ILIKE $1)
            ORDER BY start_time ASC
            "#,
        )
        .bind(&search_pattern)
        .fetch_all(pool)
        .await,

        (None, false) => sqlx::query_as::<_, Event>(
            r#"
            SELECT id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
            FROM events
            WHERE status = 'Published'
            ORDER BY start_time ASC
            "#,
        )
        .fetch_all(pool)
        .await,
    }
}

pub async fn find_event_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
        FROM events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn create_event(
    pool: &PgPool,
    organizer_id: Uuid,
    title: &str,
    description: Option<&str>,
    category: &str,
    venue_name: &str,
    venue_address: &str,
    start_time: chrono::DateTime<chrono::Utc>,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events (organizer_id, title, description, category, venue_name, venue_address, start_time, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'Published')
        RETURNING id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
        "#,
    )
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .fetch_one(pool)
    .await
}

pub async fn list_organizer_events(
    pool: &PgPool,
    organizer_id: Uuid,
) -> Result<Vec<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT id, organizer_id, title, description, category, venue_name, venue_address, start_time, status, created_at
        FROM events
        WHERE organizer_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(organizer_id)
    .fetch_all(pool)
    .await
}
