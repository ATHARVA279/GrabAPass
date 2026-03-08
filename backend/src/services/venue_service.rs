use axum::http::StatusCode;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        AssignSeatCategoryRequest, CategoryInfo, CreateVenueRequest, RowLayout,
        SeatingMode, SeatLayout, SeatStatus, SectionLayout, SeatLayoutResponse,
        StageOrientation, VenueTemplate,
    },
    repositories::venue_repository,
};

type ServiceResult<T> = Result<T, (StatusCode, String)>;

fn db_err(e: sqlx::Error) -> (StatusCode, String) {
    tracing::error!("DB error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())
}

fn not_found(msg: &str) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, msg.to_string())
}

// ─── Create a full venue template in one transaction ─────────────────────────

pub async fn create_venue_template(
    state: &AppState,
    organizer_id: Uuid,
    req: CreateVenueRequest,
) -> ServiceResult<VenueTemplate> {
    let pool = &state.pool;

    // Validate
    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Venue name is required".into()));
    }
    if req.sections.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "At least one section is required".into()));
    }
    for (si, section) in req.sections.iter().enumerate() {
        if section.name.trim().is_empty() {
            return Err((StatusCode::BAD_REQUEST, format!("Section {} name is required", si + 1)));
        }
        if section.rows.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Section '{}' must have at least one row", section.name),
            ));
        }
        for row in &section.rows {
            if row.seat_count < 1 || row.seat_count > 200 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Row '{}' seat count must be 1–200", row.row_label),
                ));
            }
        }
    }

    let stage_label = req.stage_label.as_deref().unwrap_or("STAGE");
    let orientation = req.orientation.unwrap_or(StageOrientation::North);

    // Create the template header
    let template = venue_repository::create_venue_template(
        pool,
        organizer_id,
        req.name.trim(),
        req.description.as_deref(),
        stage_label,
        &orientation,
    )
    .await
    .map_err(db_err)?;

    // Create sections → rows → seats
    for (si, section_req) in req.sections.into_iter().enumerate() {
        let color = section_req
            .color_hex
            .as_deref()
            .unwrap_or("#4A90D9");

        let section = venue_repository::create_section(
            pool,
            template.id,
            section_req.name.trim(),
            si as i32,
            color,
        )
        .await
        .map_err(db_err)?;

        for (ri, row_req) in section_req.rows.into_iter().enumerate() {
            let row = venue_repository::create_row(
                pool,
                section.id,
                &row_req.row_label,
                row_req.seat_count,
                ri as i32,
            )
            .await
            .map_err(db_err)?;

            // Generate seats 1..=seat_count, optionally overriding metadata per seat
            let seat_meta: HashMap<i32, _> = row_req
                .seats
                .unwrap_or_default()
                .into_iter()
                .map(|s| (s.seat_number, s))
                .collect();

            for seat_num in 1..=row_req.seat_count {
                let meta = seat_meta.get(&seat_num);
                let seat_label = format!("{}{}", row_req.row_label, seat_num);

                venue_repository::create_seat(
                    pool,
                    row.id,
                    seat_num,
                    &seat_label,
                    meta.and_then(|m| m.is_accessible).unwrap_or(false),
                    meta.and_then(|m| m.is_aisle).unwrap_or(false),
                    meta.and_then(|m| m.is_vip).unwrap_or(false),
                    meta.and_then(|m| m.is_companion).unwrap_or(false),
                    meta.and_then(|m| m.blocked_default).unwrap_or(false),
                )
                .await
                .map_err(db_err)?;
            }
        }
    }

    Ok(template)
}

// ─── Get a venue template (header only, no nested layout) ────────────────────

pub async fn get_venue_template(
    state: &AppState,
    id: Uuid,
    organizer_id: Uuid,
) -> ServiceResult<VenueTemplate> {
    let template = venue_repository::find_venue_template(&state.pool, id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Venue template not found"))?;

    if template.organizer_id != organizer_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }

    Ok(template)
}

// ─── List venue templates for organizer ──────────────────────────────────────

pub async fn list_venue_templates(
    state: &AppState,
    organizer_id: Uuid,
) -> ServiceResult<Vec<VenueTemplate>> {
    venue_repository::list_organizer_venue_templates(&state.pool, organizer_id)
        .await
        .map_err(db_err)
}

// ─── Assign seat categories to an event's sections ───────────────────────────

pub async fn assign_seat_categories(
    state: &AppState,
    event_id: Uuid,
    categories: Vec<AssignSeatCategoryRequest>,
) -> ServiceResult<()> {
    let pool = &state.pool;
    for cat in categories {
        let color = cat.color_hex.as_deref().unwrap_or("#4A90D9");
        venue_repository::upsert_seat_category(
            pool,
            event_id,
            cat.section_id,
            &cat.name,
            cat.price,
            color,
        )
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

// ─── Initialise seat inventory for an event ──────────────────────────────────
// Called when an event is created with a venue_template_id.

pub async fn initialise_event_inventory(
    state: &AppState,
    event_id: Uuid,
    venue_template_id: Uuid,
) -> ServiceResult<()> {
    let pool = &state.pool;

    let sections = venue_repository::list_sections_for_template(pool, venue_template_id)
        .await
        .map_err(db_err)?;

    let section_ids: Vec<Uuid> = sections.iter().map(|s| s.id).collect();
    let rows = venue_repository::list_rows_for_sections(pool, &section_ids)
        .await
        .map_err(db_err)?;

    let row_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let seats = venue_repository::list_seats_for_rows(pool, &row_ids)
        .await
        .map_err(db_err)?;

    let seat_ids: Vec<Uuid> = seats.iter().map(|s| s.id).collect();
    let blocked: Vec<bool> = seats.iter().map(|s| s.blocked_default).collect();

    venue_repository::initialise_seat_inventory(pool, event_id, &seat_ids, &blocked)
        .await
        .map_err(db_err)?;

    Ok(())
}

// ─── Build the frontend-ready seat layout response ───────────────────────────

pub async fn get_seat_layout(
    state: &AppState,
    event_id: Uuid,
) -> ServiceResult<SeatLayoutResponse> {
    let pool = &state.pool;

    // Load event to get title + venue_template_id + seating_mode
    let event = crate::repositories::event_repository::find_event_by_id(pool, event_id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Event not found"))?;

    let template_id = event
        .venue_template_id
        .ok_or_else(|| (StatusCode::NOT_FOUND, "This event has no seating layout".into()))?;

    let template = venue_repository::find_venue_template(pool, template_id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Venue template not found"))?;

    // Load all sections / rows / seats
    let sections = venue_repository::list_sections_for_template(pool, template_id)
        .await
        .map_err(db_err)?;

    let section_ids: Vec<Uuid> = sections.iter().map(|s| s.id).collect();
    let rows = venue_repository::list_rows_for_sections(pool, &section_ids)
        .await
        .map_err(db_err)?;

    let row_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let seats = venue_repository::list_seats_for_rows(pool, &row_ids)
        .await
        .map_err(db_err)?;

    // Load inventory map: seat_id → SeatStatus
    let inventory = venue_repository::list_inventory_for_event(pool, event_id)
        .await
        .map_err(db_err)?;
    let inventory_map: HashMap<Uuid, SeatStatus> = inventory
        .into_iter()
        .map(|inv| (inv.seat_id, inv.status))
        .collect();

    // Load category map: section_id → CategoryInfo
    let categories = venue_repository::list_categories_for_event(pool, event_id)
        .await
        .map_err(db_err)?;
    let category_map: HashMap<Uuid, CategoryInfo> = categories
        .into_iter()
        .map(|c| {
            (
                c.section_id,
                CategoryInfo {
                    name: c.name,
                    price: c.price,
                    color_hex: c.color_hex,
                },
            )
        })
        .collect();

    // Group seats by row_id, rows by section_id
    let mut seats_by_row: HashMap<Uuid, Vec<&_>> = HashMap::new();
    for seat in &seats {
        seats_by_row.entry(seat.row_id).or_default().push(seat);
    }

    let mut rows_by_section: HashMap<Uuid, Vec<&_>> = HashMap::new();
    for row in &rows {
        rows_by_section.entry(row.section_id).or_default().push(row);
    }

    // Assemble response
    let section_layouts: Vec<SectionLayout> = sections
        .into_iter()
        .map(|section| {
            let mut section_rows: Vec<RowLayout> = rows_by_section
                .get(&section.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|row| {
                    let mut row_seats: Vec<SeatLayout> = seats_by_row
                        .get(&row.id)
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .map(|seat| {
                            let status = inventory_map
                                .get(&seat.id)
                                .cloned()
                                .unwrap_or(SeatStatus::Available);
                            SeatLayout {
                                id: seat.id,
                                seat_number: seat.seat_number,
                                seat_label: seat.seat_label.clone(),
                                status,
                                is_accessible: seat.is_accessible,
                                is_aisle: seat.is_aisle,
                                is_vip: seat.is_vip,
                                is_companion: seat.is_companion,
                            }
                        })
                        .collect();
                    row_seats.sort_by_key(|s| s.seat_number);

                    RowLayout {
                        id: row.id,
                        row_label: row.row_label.clone(),
                        display_order: row.display_order,
                        seats: row_seats,
                    }
                })
                .collect();
            section_rows.sort_by_key(|r| r.display_order);

            SectionLayout {
                id: section.id,
                name: section.name,
                display_order: section.display_order,
                category: category_map.get(&section.id).cloned(),
                rows: section_rows,
            }
        })
        .collect();

    let mut sorted_sections = section_layouts;
    sorted_sections.sort_by_key(|s| s.display_order);

    Ok(SeatLayoutResponse {
        event_id,
        event_title: event.title,
        stage_label: template.stage_label,
        orientation: template.orientation,
        seating_mode: event.seating_mode,
        sections: sorted_sections,
    })
}

// ─── Derive seating_mode from the request ────────────────────────────────────

pub fn resolve_seating_mode(
    explicit: Option<SeatingMode>,
    has_template: bool,
) -> Option<SeatingMode> {
    match (explicit, has_template) {
        (Some(m), _) => Some(m),
        (None, true) => Some(SeatingMode::Reserved),
        (None, false) => None,
    }
}
