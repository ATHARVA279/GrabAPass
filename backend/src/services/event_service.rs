use axum::http::StatusCode;
use chrono::Utc;
use sqlx::types::Json;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        AssignGateStaffRequest, CreateEventRequest, Event, EventAvailabilityResponse,
        EventDetailsResponse, EventImagesResponse, EventPricingResponse, EventStatus,
        EventTicketTier, EventVenueInput, GateStaffSummary, OrganizerDashboardSummaryResponse,
        PublicEvent,
    },
    repositories::{auth_repository, event_repository, event_venue_repository},
    services::event_venue_service,
    services::suspicious_activity_service::SuspiciousActivityService,
    services::venue_service,
};

pub async fn list_published_events(
    state: &AppState,
    category: Option<&str>,
    search: &str,
) -> Result<Vec<PublicEvent>, (StatusCode, String)> {
    event_repository::list_published_events(&state.pool, category, search)
        .await
        .map_err(internal_error)
}

pub async fn get_event(state: &AppState, id: Uuid) -> Result<Event, (StatusCode, String)> {
    let event = event_repository::find_event_by_id(&state.pool, id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;

    if event.status != EventStatus::Published {
        return Err((StatusCode::NOT_FOUND, "Event not found".to_string()));
    }

    Ok(event)
}

pub async fn get_event_details(
    state: &AppState,
    id: Uuid,
) -> Result<EventDetailsResponse, (StatusCode, String)> {
    let event = get_event(state, id).await?;
    let venue = match event.venue_id {
        Some(venue_id) => event_venue_repository::find_event_venue_by_id(&state.pool, venue_id)
            .await
            .map_err(internal_error)?,
        None => None,
    };
    let tiers = event_repository::list_event_ticket_tiers(&state.pool, id)
        .await
        .map_err(internal_error)?;
    let pricing = event_repository::get_event_price_summary(&state.pool, id)
        .await
        .map_err(internal_error)?;
    let availability = event_repository::get_event_availability_summary(&state.pool, id)
        .await
        .map_err(internal_error)?;
    let all_images = event.image_gallery.0.clone();
    let hero = all_images.first().cloned().or_else(|| event.image_url.clone());
    let gallery = if all_images.len() > 1 {
        all_images[1..].to_vec()
    } else {
        Vec::new()
    };

    let has_reserved_seating = event.venue_template_id.is_some();

    Ok(EventDetailsResponse {
        event,
        venue,
        images: EventImagesResponse { hero, gallery },
        pricing: EventPricingResponse {
            min_price: pricing.min_price,
            max_price: pricing.max_price,
            currency: "INR".to_string(),
            tiers,
            has_reserved_seating,
        },
        availability: EventAvailabilityResponse {
            total: availability.total,
            sold: availability.sold,
            held: availability.held,
            available: availability.available,
            sold_percentage: availability.sold_percentage,
            status: availability.status,
        },
    })
}

pub async fn get_organizer_event(
    state: &AppState,
    organizer_id: Uuid,
    event_id: Uuid,
) -> Result<Event, (StatusCode, String)> {
    let event = event_repository::find_event_by_id(&state.pool, event_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;

    if event.organizer_id != organizer_id {
        return Err((StatusCode::NOT_FOUND, "Event not found".to_string()));
    }

    Ok(event)
}

pub async fn create_event(
    state: &AppState,
    organizer_id: Uuid,
    payload: CreateEventRequest,
) -> Result<Event, (StatusCode, String)> {
    let (primary_image_url, image_gallery) =
        normalize_event_gallery(payload.image_url.clone(), payload.image_gallery.clone())?;
    let mut tx = state.pool.begin().await.map_err(internal_error)?;
    let resolved_venue =
        resolve_event_venue_tx(&mut tx, organizer_id, payload.venue.as_ref(), &payload).await?;
    let seating_mode = venue_service::resolve_seating_mode(
        payload.seating_mode.clone(),
        payload.venue_template_id.is_some(),
    );

    let event = event_repository::create_event_tx(
        &mut tx,
        organizer_id,
        &payload.title,
        payload.description.as_deref(),
        &payload.category,
        resolved_venue.venue_id,
        &resolved_venue.venue_name,
        &resolved_venue.venue_address,
        payload.start_time,
        payload.venue_template_id,
        seating_mode,
        primary_image_url.as_deref(),
        &image_gallery,
        resolved_venue.venue_place_id.as_deref(),
        resolved_venue.venue_latitude,
        resolved_venue.venue_longitude,
    )
    .await
    .map_err(internal_error)?;

    // If a venue template was attached, initialise the seat inventory immediately
    if let Some(template_id) = event.venue_template_id {
        venue_service::initialise_event_inventory_tx(&mut tx, state, event.id, template_id).await?;
    }

    if let Some(ticket_tiers) = payload.ticket_tiers.as_deref() {
        event_repository::replace_event_ticket_tiers(&mut tx, event.id, ticket_tiers)
            .await
            .map_err(internal_error)?;
    }

    tx.commit().await.map_err(internal_error)?;

    Ok(event)
}

pub async fn update_event(
    state: &AppState,
    organizer_id: Uuid,
    event_id: Uuid,
    payload: CreateEventRequest,
) -> Result<Event, (StatusCode, String)> {
    let (primary_image_url, image_gallery) =
        normalize_event_gallery(payload.image_url.clone(), payload.image_gallery.clone())?;
    let mut tx = state.pool.begin().await.map_err(internal_error)?;
    let resolved_venue =
        resolve_event_venue_tx(&mut tx, organizer_id, payload.venue.as_ref(), &payload).await?;
    let seating_mode = venue_service::resolve_seating_mode(
        payload.seating_mode.clone(),
        payload.venue_template_id.is_some(),
    );

    let event = event_repository::update_event(
        &state.pool,
        event_id,
        organizer_id,
        &payload.title,
        payload.description.as_deref(),
        &payload.category,
        resolved_venue.venue_id,
        &resolved_venue.venue_name,
        &resolved_venue.venue_address,
        payload.start_time,
        payload.venue_template_id,
        seating_mode,
        primary_image_url.as_deref(),
        &image_gallery,
        resolved_venue.venue_place_id.as_deref(),
        resolved_venue.venue_latitude,
        resolved_venue.venue_longitude,
    )
    .await
    .map_err(internal_error)?
    .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;

    event_repository::replace_event_ticket_tiers(
        &mut tx,
        event.id,
        payload.ticket_tiers.as_deref().unwrap_or(&[]),
    )
    .await
    .map_err(internal_error)?;

    tx.commit().await.map_err(internal_error)?;

    Ok(event)
}

pub async fn delete_event(
    state: &AppState,
    organizer_id: Uuid,
    event_id: Uuid,
) -> Result<(), (StatusCode, String)> {
    let mut tx = state.pool.begin().await.map_err(internal_error)?;

    let rows_affected = event_repository::delete_event_transaction(&mut tx, event_id, organizer_id)
        .await
        .map_err(internal_error)?;

    if rows_affected == 0 {
        tx.rollback().await.map_err(internal_error)?;
        return Err((StatusCode::NOT_FOUND, "Event not found".to_string()));
    }

    tx.commit().await.map_err(internal_error)?;

    Ok(())
}

pub async fn cancel_event(
    state: &AppState,
    organizer_id: Uuid,
    event_id: Uuid,
) -> Result<Event, (StatusCode, String)> {
    let existing = event_repository::find_event_by_id(&state.pool, event_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;

    if existing.organizer_id != organizer_id {
        return Err((StatusCode::NOT_FOUND, "Event not found".to_string()));
    }

    if existing.status == crate::db::models::EventStatus::Cancelled {
        return Err((
            StatusCode::CONFLICT,
            "Event is already cancelled.".to_string(),
        ));
    }

    if existing.start_time <= Utc::now() {
        return Err((
            StatusCode::CONFLICT,
            "Started events cannot be cancelled.".to_string(),
        ));
    }

    let mut tx = state.pool.begin().await.map_err(internal_error)?;
    let event = event_repository::cancel_event_transaction(&mut tx, event_id, organizer_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;
    tx.commit().await.map_err(internal_error)?;

    Ok(event)
}

pub async fn list_organizer_events(
    state: &AppState,
    organizer_id: Uuid,
) -> Result<Vec<Event>, (StatusCode, String)> {
    event_repository::list_organizer_events(&state.pool, organizer_id)
        .await
        .map_err(internal_error)
}

pub async fn get_organizer_dashboard_summary(
    state: &AppState,
    organizer_id: Uuid,
) -> Result<OrganizerDashboardSummaryResponse, (StatusCode, String)> {
    let mut summary = event_repository::get_organizer_dashboard_summary(&state.pool, organizer_id)
        .await
        .map_err(internal_error)?;

    summary.suspicious_alerts =
        SuspiciousActivityService::count_recent_for_organizer(&state.pool, organizer_id).await?;
    summary.recent_alerts =
        SuspiciousActivityService::list_recent_for_organizer(&state.pool, organizer_id).await?;

    Ok(summary)
}

pub async fn list_gate_staff_users(
    state: &AppState,
) -> Result<Vec<GateStaffSummary>, (StatusCode, String)> {
    auth_repository::list_gate_staff_users(&state.pool)
        .await
        .map_err(internal_error)
}

pub async fn list_assigned_gate_staff(
    state: &AppState,
    organizer_id: Uuid,
    event_id: Uuid,
) -> Result<Vec<GateStaffSummary>, (StatusCode, String)> {
    event_repository::list_assigned_gate_staff(&state.pool, event_id, organizer_id)
        .await
        .map_err(internal_error)
}

pub async fn assign_gate_staff(
    state: &AppState,
    organizer_id: Uuid,
    event_id: Uuid,
    payload: AssignGateStaffRequest,
) -> Result<(), (StatusCode, String)> {
    let mut tx = state.pool.begin().await.map_err(internal_error)?;
    event_repository::replace_gate_staff_assignments(
        &mut tx,
        event_id,
        organizer_id,
        &payload.gate_staff_ids,
    )
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Event not found".to_string()),
        _ => internal_error(error),
    })?;
    tx.commit().await.map_err(internal_error)?;
    Ok(())
}

fn internal_error(error: sqlx::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn normalize_event_gallery(
    image_url: Option<String>,
    image_gallery: Option<Vec<String>>,
) -> Result<(Option<String>, Json<Vec<String>>), (StatusCode, String)> {
    let mut gallery = Vec::new();

    if let Some(primary) = normalize_optional_text(image_url.as_deref()) {
        gallery.push(primary);
    }

    if let Some(images) = image_gallery {
        for image in images {
            if let Some(normalized) = normalize_optional_text(Some(&image)) {
                if !gallery.iter().any(|existing| existing == &normalized) {
                    gallery.push(normalized);
                }
            }
        }
    }

    if gallery.len() > 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "You can upload up to 8 event images.".to_string(),
        ));
    }

    let primary_image = gallery.first().cloned();
    Ok((primary_image, Json(gallery)))
}

fn normalize_coordinates(
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> Result<(Option<f64>, Option<f64>), (StatusCode, String)> {
    match (latitude, longitude) {
        (Some(lat), Some(lng)) => Ok((Some(lat), Some(lng))),
        (None, None) => Ok((None, None)),
        _ => Err((
            StatusCode::BAD_REQUEST,
            "Venue latitude and longitude must both be provided together.".to_string(),
        )),
    }
}

fn normalize_required_text(value: &str, field_name: &str) -> Result<String, (StatusCode, String)> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{field_name} is required."),
        ));
    }

    Ok(normalized.to_string())
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

struct ResolvedVenueFields {
    venue_id: Option<Uuid>,
    venue_name: String,
    venue_address: String,
    venue_place_id: Option<String>,
    venue_latitude: Option<f64>,
    venue_longitude: Option<f64>,
}

async fn resolve_event_venue_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organizer_id: Uuid,
    venue_input: Option<&EventVenueInput>,
    payload: &CreateEventRequest,
) -> Result<ResolvedVenueFields, (StatusCode, String)> {
    if let Some(venue_input) = venue_input {
        let venue = event_venue_service::save_event_venue_tx(tx, organizer_id, venue_input).await?;
        return Ok(ResolvedVenueFields {
            venue_id: Some(venue.id),
            venue_name: venue.name,
            venue_address: venue.address,
            venue_place_id: Some(venue.place_id),
            venue_latitude: Some(venue.latitude),
            venue_longitude: Some(venue.longitude),
        });
    }

    let venue_name = normalize_required_text(&payload.venue_name, "Venue name")?;
    let venue_address = normalize_required_text(&payload.venue_address, "Venue address")?;
    let (venue_latitude, venue_longitude) =
        normalize_coordinates(payload.venue_latitude, payload.venue_longitude)?;

    Ok(ResolvedVenueFields {
        venue_id: None,
        venue_name,
        venue_address,
        venue_place_id: normalize_optional_text(payload.venue_place_id.as_deref()),
        venue_latitude,
        venue_longitude,
    })
}

pub async fn get_event_pulse(
    state: &AppState,
    event_id: Uuid,
) -> Result<crate::db::models::EventPulseResponse, (StatusCode, String)> {
    event_repository::get_event_pulse(&state.pool, event_id)
        .await
        .map_err(internal_error)
}

pub async fn get_event_ticket_tiers(
    state: &AppState,
    event_id: Uuid,
) -> Result<Vec<EventTicketTier>, (StatusCode, String)> {
    let event = event_repository::find_event_by_id(&state.pool, event_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Event not found".to_string()))?;

    if event.status != EventStatus::Published {
        return Err((StatusCode::NOT_FOUND, "Event not found".to_string()));
    }

    event_repository::list_event_ticket_tiers(&state.pool, event_id)
        .await
        .map_err(internal_error)
}
