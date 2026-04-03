use axum::http::StatusCode;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{EventVenue, EventVenueInput, EventVenueMatchResponse},
    repositories::event_venue_repository,
};

type ServiceResult<T> = Result<T, (StatusCode, String)>;

fn db_err(error: sqlx::Error) -> (StatusCode, String) {
    tracing::error!("DB error: {error}");
    (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())
}

fn normalize_required(value: &str, field_name: &str) -> ServiceResult<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err((StatusCode::BAD_REQUEST, format!("{field_name} is required")));
    }

    Ok(normalized.to_string())
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn validate_coordinates(latitude: f64, longitude: f64) -> ServiceResult<()> {
    if !(-90.0..=90.0).contains(&latitude) {
        return Err((StatusCode::BAD_REQUEST, "Latitude is invalid".into()));
    }

    if !(-180.0..=180.0).contains(&longitude) {
        return Err((StatusCode::BAD_REQUEST, "Longitude is invalid".into()));
    }

    Ok(())
}

pub fn validate_event_venue_input(input: &EventVenueInput) -> ServiceResult<()> {
    let _ = normalize_required(&input.name, "Venue name")?;
    let _ = normalize_required(&input.place_id, "Google Place ID")?;
    let _ = normalize_required(&input.address, "Address")?;
    let _ = normalize_required(&input.city, "City")?;
    let _ = normalize_required(&input.state, "State")?;
    let _ = normalize_required(&input.country, "Country")?;
    validate_coordinates(input.latitude, input.longitude)?;

    if let Some(capacity) = input.capacity {
        if capacity < 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Capacity cannot be negative".into(),
            ));
        }
    }

    Ok(())
}

pub async fn save_event_venue(
    state: &AppState,
    organizer_id: Uuid,
    input: EventVenueInput,
) -> ServiceResult<EventVenue> {
    validate_event_venue_input(&input)?;
    let landmark = normalize_optional(input.landmark.as_deref());

    event_venue_repository::upsert_event_venue(
        &state.pool,
        organizer_id,
        input.name.trim(),
        input.place_id.trim(),
        input.latitude,
        input.longitude,
        input.address.trim(),
        input.locality.trim(),
        input.city.trim(),
        input.state.trim(),
        input.pincode.trim(),
        input.country.trim(),
        landmark.as_deref(),
        input.capacity,
    )
    .await
    .map_err(db_err)
}

pub async fn save_event_venue_tx(
    tx: &mut Transaction<'_, Postgres>,
    organizer_id: Uuid,
    input: &EventVenueInput,
) -> ServiceResult<EventVenue> {
    validate_event_venue_input(input)?;
    let landmark = normalize_optional(input.landmark.as_deref());

    event_venue_repository::upsert_event_venue_tx(
        tx,
        organizer_id,
        input.name.trim(),
        input.place_id.trim(),
        input.latitude,
        input.longitude,
        input.address.trim(),
        input.locality.trim(),
        input.city.trim(),
        input.state.trim(),
        input.pincode.trim(),
        input.country.trim(),
        landmark.as_deref(),
        input.capacity,
    )
    .await
    .map_err(db_err)
}

pub async fn get_event_venue(state: &AppState, venue_id: Uuid) -> ServiceResult<EventVenue> {
    event_venue_repository::find_event_venue_by_id(&state.pool, venue_id)
        .await
        .map_err(db_err)?
        .ok_or((StatusCode::NOT_FOUND, "Venue not found".into()))
}

pub async fn find_event_venue_matches(
    state: &AppState,
    input: EventVenueInput,
) -> ServiceResult<EventVenueMatchResponse> {
    validate_event_venue_input(&input)?;

    let similar_venues = event_venue_repository::find_similar_event_venues(
        &state.pool,
        input.place_id.trim(),
        input.name.trim(),
        input.city.trim(),
        input.state.trim(),
        input.latitude,
        input.longitude,
        6,
    )
    .await
    .map_err(db_err)?;

    let exact_match = similar_venues
        .iter()
        .find(|venue| venue.place_id.eq_ignore_ascii_case(input.place_id.trim()))
        .cloned();

    Ok(EventVenueMatchResponse {
        exact_match,
        similar_venues,
    })
}
