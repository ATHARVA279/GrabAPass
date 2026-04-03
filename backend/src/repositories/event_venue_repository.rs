use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::db::models::EventVenue;

pub async fn find_event_venue_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<EventVenue>, sqlx::Error> {
    sqlx::query_as::<_, EventVenue>(
        r#"
        SELECT
            id,
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity,
            created_at,
            updated_at
        FROM event_venues
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn find_event_venue_by_place_id(
    pool: &PgPool,
    place_id: &str,
) -> Result<Option<EventVenue>, sqlx::Error> {
    sqlx::query_as::<_, EventVenue>(
        r#"
        SELECT
            id,
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity,
            created_at,
            updated_at
        FROM event_venues
        WHERE place_id = $1
        "#,
    )
    .bind(place_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_similar_event_venues(
    pool: &PgPool,
    place_id: &str,
    name: &str,
    city: &str,
    state: &str,
    latitude: f64,
    longitude: f64,
    limit: i64,
) -> Result<Vec<EventVenue>, sqlx::Error> {
    sqlx::query_as::<_, EventVenue>(
        r#"
        SELECT
            id,
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity,
            created_at,
            updated_at
        FROM event_venues
        WHERE place_id = $1
           OR (
                LOWER(name) = LOWER($2)
                AND LOWER(city) = LOWER($3)
                AND LOWER(state) = LOWER($4)
             )
           OR (
                ABS(latitude - $5) <= 0.01
                AND ABS(longitude - $6) <= 0.01
             )
        ORDER BY
            CASE
                WHEN place_id = $1 THEN 0
                WHEN LOWER(name) = LOWER($2) AND LOWER(city) = LOWER($3) AND LOWER(state) = LOWER($4) THEN 1
                ELSE 2
            END,
            updated_at DESC
        LIMIT $7
        "#,
    )
    .bind(place_id)
    .bind(name)
    .bind(city)
    .bind(state)
    .bind(latitude)
    .bind(longitude)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn upsert_event_venue(
    pool: &PgPool,
    created_by: Uuid,
    name: &str,
    place_id: &str,
    latitude: f64,
    longitude: f64,
    address: &str,
    locality: &str,
    city: &str,
    state: &str,
    pincode: &str,
    country: &str,
    landmark: Option<&str>,
    capacity: Option<i32>,
) -> Result<EventVenue, sqlx::Error> {
    sqlx::query_as::<_, EventVenue>(
        r#"
        INSERT INTO event_venues (
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULLIF($12, ''), $13)
        ON CONFLICT (place_id)
        DO UPDATE SET
            name = EXCLUDED.name,
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            address = EXCLUDED.address,
            locality = EXCLUDED.locality,
            city = EXCLUDED.city,
            state = EXCLUDED.state,
            pincode = EXCLUDED.pincode,
            country = EXCLUDED.country,
            landmark = COALESCE(NULLIF(EXCLUDED.landmark, ''), event_venues.landmark),
            capacity = COALESCE(EXCLUDED.capacity, event_venues.capacity),
            updated_at = NOW()
        RETURNING
            id,
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity,
            created_at,
            updated_at
        "#,
    )
    .bind(created_by)
    .bind(name)
    .bind(place_id)
    .bind(latitude)
    .bind(longitude)
    .bind(address)
    .bind(locality)
    .bind(city)
    .bind(state)
    .bind(pincode)
    .bind(country)
    .bind(landmark)
    .bind(capacity)
    .fetch_one(pool)
    .await
}

pub async fn upsert_event_venue_tx(
    tx: &mut Transaction<'_, Postgres>,
    created_by: Uuid,
    name: &str,
    place_id: &str,
    latitude: f64,
    longitude: f64,
    address: &str,
    locality: &str,
    city: &str,
    state: &str,
    pincode: &str,
    country: &str,
    landmark: Option<&str>,
    capacity: Option<i32>,
) -> Result<EventVenue, sqlx::Error> {
    sqlx::query_as::<_, EventVenue>(
        r#"
        INSERT INTO event_venues (
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULLIF($12, ''), $13)
        ON CONFLICT (place_id)
        DO UPDATE SET
            name = EXCLUDED.name,
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            address = EXCLUDED.address,
            locality = EXCLUDED.locality,
            city = EXCLUDED.city,
            state = EXCLUDED.state,
            pincode = EXCLUDED.pincode,
            country = EXCLUDED.country,
            landmark = COALESCE(NULLIF(EXCLUDED.landmark, ''), event_venues.landmark),
            capacity = COALESCE(EXCLUDED.capacity, event_venues.capacity),
            updated_at = NOW()
        RETURNING
            id,
            created_by,
            name,
            place_id,
            latitude,
            longitude,
            address,
            locality,
            city,
            state,
            pincode,
            country,
            landmark,
            capacity,
            created_at,
            updated_at
        "#,
    )
    .bind(created_by)
    .bind(name)
    .bind(place_id)
    .bind(latitude)
    .bind(longitude)
    .bind(address)
    .bind(locality)
    .bind(city)
    .bind(state)
    .bind(pincode)
    .bind(country)
    .bind(landmark)
    .bind(capacity)
    .fetch_one(&mut **tx)
    .await
}
