use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::db::models::{
    EventSeatCategory, EventSeatInventory, SeatStatus, StageOrientation, VenueRow, VenueSeat,
    VenueSection, VenueTemplate,
};

// ─── Venue template ───────────────────────────────────────────────────────────

pub async fn create_venue_template(
    pool: &PgPool,
    organizer_id: Uuid,
    name: &str,
    description: Option<&str>,
    stage_label: &str,
    orientation: &StageOrientation,
) -> Result<VenueTemplate, sqlx::Error> {
    sqlx::query_as::<_, VenueTemplate>(
        r#"
        INSERT INTO venue_templates (organizer_id, name, description, stage_label, orientation)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, organizer_id, name, description, stage_label, orientation, created_at
        "#,
    )
    .bind(organizer_id)
    .bind(name)
    .bind(description)
    .bind(stage_label)
    .bind(orientation)
    .fetch_one(pool)
    .await
}

pub async fn create_venue_template_tx(
    tx: &mut Transaction<'_, Postgres>,
    organizer_id: Uuid,
    name: &str,
    description: Option<&str>,
    stage_label: &str,
    orientation: &StageOrientation,
) -> Result<VenueTemplate, sqlx::Error> {
    sqlx::query_as::<_, VenueTemplate>(
        r#"
        INSERT INTO venue_templates (organizer_id, name, description, stage_label, orientation)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, organizer_id, name, description, stage_label, orientation, created_at
        "#,
    )
    .bind(organizer_id)
    .bind(name)
    .bind(description)
    .bind(stage_label)
    .bind(orientation)
    .fetch_one(&mut **tx)
    .await
}

pub async fn find_venue_template(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<VenueTemplate>, sqlx::Error> {
    sqlx::query_as::<_, VenueTemplate>(
        r#"
        SELECT id, organizer_id, name, description, stage_label, orientation, created_at
        FROM venue_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_organizer_venue_templates(
    pool: &PgPool,
    organizer_id: Uuid,
) -> Result<Vec<VenueTemplate>, sqlx::Error> {
    sqlx::query_as::<_, VenueTemplate>(
        r#"
        SELECT id, organizer_id, name, description, stage_label, orientation, created_at
        FROM venue_templates
        WHERE organizer_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(organizer_id)
    .fetch_all(pool)
    .await
}

// ─── Sections ─────────────────────────────────────────────────────────────────

pub async fn create_section(
    pool: &PgPool,
    venue_template_id: Uuid,
    name: &str,
    display_order: i32,
    color_hex: &str,
) -> Result<VenueSection, sqlx::Error> {
    sqlx::query_as::<_, VenueSection>(
        r#"
        INSERT INTO venue_sections (venue_template_id, name, display_order, color_hex)
        VALUES ($1, $2, $3, $4)
        RETURNING id, venue_template_id, name, display_order, color_hex
        "#,
    )
    .bind(venue_template_id)
    .bind(name)
    .bind(display_order)
    .bind(color_hex)
    .fetch_one(pool)
    .await
}

pub async fn create_section_tx(
    tx: &mut Transaction<'_, Postgres>,
    venue_template_id: Uuid,
    name: &str,
    display_order: i32,
    color_hex: &str,
) -> Result<VenueSection, sqlx::Error> {
    sqlx::query_as::<_, VenueSection>(
        r#"
        INSERT INTO venue_sections (venue_template_id, name, display_order, color_hex)
        VALUES ($1, $2, $3, $4)
        RETURNING id, venue_template_id, name, display_order, color_hex
        "#,
    )
    .bind(venue_template_id)
    .bind(name)
    .bind(display_order)
    .bind(color_hex)
    .fetch_one(&mut **tx)
    .await
}

pub async fn list_sections_for_template(
    pool: &PgPool,
    venue_template_id: Uuid,
) -> Result<Vec<VenueSection>, sqlx::Error> {
    sqlx::query_as::<_, VenueSection>(
        r#"
        SELECT id, venue_template_id, name, display_order, color_hex
        FROM venue_sections
        WHERE venue_template_id = $1
        ORDER BY display_order ASC, name ASC
        "#,
    )
    .bind(venue_template_id)
    .fetch_all(pool)
    .await
}

// ─── Rows ─────────────────────────────────────────────────────────────────────

pub async fn create_row(
    pool: &PgPool,
    section_id: Uuid,
    row_label: &str,
    seat_count: i32,
    display_order: i32,
) -> Result<VenueRow, sqlx::Error> {
    sqlx::query_as::<_, VenueRow>(
        r#"
        INSERT INTO venue_rows (section_id, row_label, seat_count, display_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id, section_id, row_label, seat_count, display_order
        "#,
    )
    .bind(section_id)
    .bind(row_label)
    .bind(seat_count)
    .bind(display_order)
    .fetch_one(pool)
    .await
}

pub async fn create_row_tx(
    tx: &mut Transaction<'_, Postgres>,
    section_id: Uuid,
    row_label: &str,
    seat_count: i32,
    display_order: i32,
) -> Result<VenueRow, sqlx::Error> {
    sqlx::query_as::<_, VenueRow>(
        r#"
        INSERT INTO venue_rows (section_id, row_label, seat_count, display_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id, section_id, row_label, seat_count, display_order
        "#,
    )
    .bind(section_id)
    .bind(row_label)
    .bind(seat_count)
    .bind(display_order)
    .fetch_one(&mut **tx)
    .await
}

pub async fn list_rows_for_sections(
    pool: &PgPool,
    section_ids: &[Uuid],
) -> Result<Vec<VenueRow>, sqlx::Error> {
    sqlx::query_as::<_, VenueRow>(
        r#"
        SELECT id, section_id, row_label, seat_count, display_order
        FROM venue_rows
        WHERE section_id = ANY($1)
        ORDER BY section_id, display_order ASC, row_label ASC
        "#,
    )
    .bind(section_ids)
    .fetch_all(pool)
    .await
}

// ─── Seats ────────────────────────────────────────────────────────────────────

pub async fn create_seat(
    pool: &PgPool,
    row_id: Uuid,
    seat_number: i32,
    seat_label: &str,
    is_accessible: bool,
    is_aisle: bool,
    is_vip: bool,
    is_companion: bool,
    blocked_default: bool,
) -> Result<VenueSeat, sqlx::Error> {
    sqlx::query_as::<_, VenueSeat>(
        r#"
        INSERT INTO venue_seats
            (row_id, seat_number, seat_label, is_accessible, is_aisle, is_vip, is_companion, blocked_default)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, row_id, seat_number, seat_label, is_accessible, is_aisle, is_vip, is_companion, blocked_default
        "#,
    )
    .bind(row_id)
    .bind(seat_number)
    .bind(seat_label)
    .bind(is_accessible)
    .bind(is_aisle)
    .bind(is_vip)
    .bind(is_companion)
    .bind(blocked_default)
    .fetch_one(pool)
    .await
}

pub async fn create_seat_tx(
    tx: &mut Transaction<'_, Postgres>,
    row_id: Uuid,
    seat_number: i32,
    seat_label: &str,
    is_accessible: bool,
    is_aisle: bool,
    is_vip: bool,
    is_companion: bool,
    blocked_default: bool,
) -> Result<VenueSeat, sqlx::Error> {
    sqlx::query_as::<_, VenueSeat>(
        r#"
        INSERT INTO venue_seats
            (row_id, seat_number, seat_label, is_accessible, is_aisle, is_vip, is_companion, blocked_default)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, row_id, seat_number, seat_label, is_accessible, is_aisle, is_vip, is_companion, blocked_default
        "#,
    )
    .bind(row_id)
    .bind(seat_number)
    .bind(seat_label)
    .bind(is_accessible)
    .bind(is_aisle)
    .bind(is_vip)
    .bind(is_companion)
    .bind(blocked_default)
    .fetch_one(&mut **tx)
    .await
}

pub async fn list_seats_for_rows(
    pool: &PgPool,
    row_ids: &[Uuid],
) -> Result<Vec<VenueSeat>, sqlx::Error> {
    sqlx::query_as::<_, VenueSeat>(
        r#"
        SELECT id, row_id, seat_number, seat_label, is_accessible, is_aisle, is_vip, is_companion, blocked_default
        FROM venue_seats
        WHERE row_id = ANY($1)
        ORDER BY row_id, seat_number ASC
        "#,
    )
    .bind(row_ids)
    .fetch_all(pool)
    .await
}

// ─── Event seat categories ────────────────────────────────────────────────────

pub async fn upsert_seat_category(
    pool: &PgPool,
    event_id: Uuid,
    section_id: Uuid,
    name: &str,
    price: f64,
    color_hex: &str,
) -> Result<EventSeatCategory, sqlx::Error> {
    if let Some(existing) = sqlx::query_as::<_, EventSeatCategory>(
        r#"
        UPDATE event_seat_categories
        SET name = $3,
            price = $4,
            color_hex = $5
        WHERE event_id = $1
          AND section_id = $2
        RETURNING id, event_id, section_id, name, price::float8, color_hex
        "#,
    )
    .bind(event_id)
    .bind(section_id)
    .bind(name)
    .bind(price)
    .bind(color_hex)
    .fetch_optional(pool)
    .await?
    {
        return Ok(existing);
    }

    sqlx::query_as::<_, EventSeatCategory>(
        r#"
        INSERT INTO event_seat_categories (event_id, section_id, name, price, color_hex)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, event_id, section_id, name, price::float8, color_hex
        "#,
    )
    .bind(event_id)
    .bind(section_id)
    .bind(name)
    .bind(price)
    .bind(color_hex)
    .fetch_one(pool)
    .await
}

pub async fn list_categories_for_event(
    pool: &PgPool,
    event_id: Uuid,
) -> Result<Vec<EventSeatCategory>, sqlx::Error> {
    sqlx::query_as::<_, EventSeatCategory>(
        r#"
        SELECT id, event_id, section_id, name, price::float8, color_hex
        FROM event_seat_categories
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .fetch_all(pool)
    .await
}

// ─── Event seat inventory ─────────────────────────────────────────────────────

/// Bulk-insert one inventory row per seat from the venue template.
/// Uses INSERT … ON CONFLICT DO NOTHING so it is safe to call more than once.
pub async fn initialise_seat_inventory(
    pool: &PgPool,
    event_id: Uuid,
    seat_ids: &[Uuid],
    blocked_defaults: &[bool],
) -> Result<u64, sqlx::Error> {
    let mut inserted = 0;
    for (&seat_id, &blocked) in seat_ids.iter().zip(blocked_defaults.iter()) {
        let status = if blocked {
            SeatStatus::Blocked
        } else {
            SeatStatus::Available
        };

        let result = sqlx::query(
            r#"
            INSERT INTO event_seat_inventory (event_id, seat_id, status)
            SELECT $1, $2, $3
            WHERE NOT EXISTS (
                SELECT 1
                FROM event_seat_inventory
                WHERE event_id = $1 AND seat_id = $2
            )
            "#,
        )
        .bind(event_id)
        .bind(seat_id)
        .bind(status)
        .execute(pool)
        .await?;

        inserted += result.rows_affected();
    }

    Ok(inserted)
}

pub async fn initialise_seat_inventory_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    seat_ids: &[Uuid],
    blocked_defaults: &[bool],
) -> Result<u64, sqlx::Error> {
    let mut inserted = 0;
    for (&seat_id, &blocked) in seat_ids.iter().zip(blocked_defaults.iter()) {
        let status = if blocked {
            SeatStatus::Blocked
        } else {
            SeatStatus::Available
        };

        let result = sqlx::query(
            r#"
            INSERT INTO event_seat_inventory (event_id, seat_id, status)
            SELECT $1, $2, $3
            WHERE NOT EXISTS (
                SELECT 1
                FROM event_seat_inventory
                WHERE event_id = $1 AND seat_id = $2
            )
            "#,
        )
        .bind(event_id)
        .bind(seat_id)
        .bind(status)
        .execute(&mut **tx)
        .await?;

        inserted += result.rows_affected();
    }

    Ok(inserted)
}

pub async fn list_inventory_for_event(
    pool: &PgPool,
    event_id: Uuid,
) -> Result<Vec<EventSeatInventory>, sqlx::Error> {
    sqlx::query_as::<_, EventSeatInventory>(
        r#"
        SELECT
            esi.id,
            esi.event_id,
            esi.seat_id,
            CASE 
                WHEN esi.status = 'Held'::seat_status AND sh.expires_at <= NOW() THEN 'Available'::seat_status
                ELSE esi.status
            END as "status"
        FROM event_seat_inventory esi
        LEFT JOIN seat_holds sh ON esi.seat_id = sh.seat_id AND esi.event_id = sh.event_id
        WHERE esi.event_id = $1
        "#,
    )
    .bind(event_id)
    .fetch_all(pool)
    .await
}
