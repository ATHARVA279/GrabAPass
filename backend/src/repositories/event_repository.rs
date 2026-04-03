use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::db::models::{
    CreateEventTicketTierRequest, Event, EventTicketTier, GateStaffSummary,
    OrganizerDashboardSummaryResponse, OrganizerEventDashboardSummary, PublicEvent, SeatingMode,
};

pub async fn list_published_events(
    pool: &PgPool,
    category: Option<&str>,
    search: &str,
) -> Result<Vec<PublicEvent>, sqlx::Error> {
    let query = r#"
        SELECT
            e.id,
            e.organizer_id,
            e.title,
            e.description,
            e.category,
            e.venue_name,
            e.venue_address,
            e.start_time,
            e.status,
            e.created_at,
            e.venue_id,
            e.venue_template_id,
            e.seating_mode,
            e.image_url,
            e.image_gallery,
            e.venue_place_id,
            e.venue_latitude,
            e.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity,
            price_stats.min_price,
            price_stats.max_price
        FROM events e
        LEFT JOIN event_venues v ON v.id = e.venue_id
        LEFT JOIN LATERAL (
            SELECT
                MIN(price)::float8 AS min_price,
                MAX(price)::float8 AS max_price
            FROM (
                SELECT esc.price
                FROM event_seat_categories esc
                WHERE esc.event_id = e.id
                UNION ALL
                SELECT ett.price
                FROM event_ticket_tiers ett
                WHERE ett.event_id = e.id
            ) AS event_prices
        ) price_stats ON TRUE
        WHERE e.status = 'Published'
          AND ($1::varchar IS NULL OR e.category = $1)
          AND (
              $2::varchar = ''
              OR e.title ILIKE $3
              OR COALESCE(e.description, '') ILIKE $3
          )
        ORDER BY e.start_time ASC
    "#;

    let search_pattern = format!("%{search}%");
    sqlx::query_as::<_, PublicEvent>(query)
        .bind(category)
        .bind(search)
        .bind(&search_pattern)
        .fetch_all(pool)
        .await
}

pub async fn list_event_ticket_tiers(
    pool: &PgPool,
    event_id: Uuid,
) -> Result<Vec<EventTicketTier>, sqlx::Error> {
    sqlx::query_as::<_, EventTicketTier>(
        r#"
        SELECT id, event_id, name, price::float8 AS price, capacity, color_hex, created_at
        FROM event_ticket_tiers
        WHERE event_id = $1
        ORDER BY price ASC, created_at ASC
        "#,
    )
    .bind(event_id)
    .fetch_all(pool)
    .await
}

pub async fn replace_event_ticket_tiers(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    tiers: &[CreateEventTicketTierRequest],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM event_ticket_tiers WHERE event_id = $1")
        .bind(event_id)
        .execute(&mut **tx)
        .await?;

    for tier in tiers {
        sqlx::query(
            r#"
            INSERT INTO event_ticket_tiers (event_id, name, price, capacity, color_hex)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(event_id)
        .bind(tier.name.trim())
        .bind(tier.price)
        .bind(tier.capacity)
        .bind(tier.color_hex.as_deref().unwrap_or("#4A90D9"))
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn find_event_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT
            e.id,
            e.organizer_id,
            e.title,
            e.description,
            e.category,
            e.venue_name,
            e.venue_address,
            e.start_time,
            e.status,
            e.created_at,
            e.venue_id,
            e.venue_template_id,
            e.seating_mode,
            e.image_url,
            e.image_gallery,
            e.venue_place_id,
            e.venue_latitude,
            e.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity
        FROM events e
        LEFT JOIN event_venues v ON v.id = e.venue_id
        WHERE e.id = $1
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
    venue_id: Option<Uuid>,
    venue_name: &str,
    venue_address: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    venue_template_id: Option<Uuid>,
    seating_mode: Option<SeatingMode>,
    image_url: Option<&str>,
    image_gallery: &Json<Vec<String>>,
    venue_place_id: Option<&str>,
    venue_latitude: Option<f64>,
    venue_longitude: Option<f64>,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        WITH inserted AS (
            INSERT INTO events
                (organizer_id, title, description, category, venue_id, venue_name, venue_address,
                 start_time, status, venue_template_id, seating_mode, image_url, image_gallery,
                 venue_place_id, venue_latitude, venue_longitude)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'Published', $9, $10::text::seating_mode, $11, $12, $13, $14, $15)
            RETURNING *
        )
        SELECT
            inserted.id,
            inserted.organizer_id,
            inserted.title,
            inserted.description,
            inserted.category,
            inserted.venue_name,
            inserted.venue_address,
            inserted.start_time,
            inserted.status,
            inserted.created_at,
            inserted.venue_id,
            inserted.venue_template_id,
            inserted.seating_mode,
            inserted.image_url,
            inserted.image_gallery,
            inserted.venue_place_id,
            inserted.venue_latitude,
            inserted.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity
        FROM inserted
        LEFT JOIN event_venues v ON v.id = inserted.venue_id
        "#,
    )
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_id)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .bind(venue_template_id)
    .bind(seating_mode)
    .bind(image_url)
    .bind(image_gallery)
    .bind(venue_place_id)
    .bind(venue_latitude)
    .bind(venue_longitude)
    .fetch_one(pool)
    .await
}

pub async fn create_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    organizer_id: Uuid,
    title: &str,
    description: Option<&str>,
    category: &str,
    venue_id: Option<Uuid>,
    venue_name: &str,
    venue_address: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    venue_template_id: Option<Uuid>,
    seating_mode: Option<SeatingMode>,
    image_url: Option<&str>,
    image_gallery: &Json<Vec<String>>,
    venue_place_id: Option<&str>,
    venue_latitude: Option<f64>,
    venue_longitude: Option<f64>,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        WITH inserted AS (
            INSERT INTO events
                (organizer_id, title, description, category, venue_id, venue_name, venue_address,
                 start_time, status, venue_template_id, seating_mode, image_url, image_gallery,
                 venue_place_id, venue_latitude, venue_longitude)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'Published', $9, $10::text::seating_mode, $11, $12, $13, $14, $15)
            RETURNING *
        )
        SELECT
            inserted.id,
            inserted.organizer_id,
            inserted.title,
            inserted.description,
            inserted.category,
            inserted.venue_name,
            inserted.venue_address,
            inserted.start_time,
            inserted.status,
            inserted.created_at,
            inserted.venue_id,
            inserted.venue_template_id,
            inserted.seating_mode,
            inserted.image_url,
            inserted.image_gallery,
            inserted.venue_place_id,
            inserted.venue_latitude,
            inserted.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity
        FROM inserted
        LEFT JOIN event_venues v ON v.id = inserted.venue_id
        "#,
    )
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_id)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .bind(venue_template_id)
    .bind(seating_mode)
    .bind(image_url)
    .bind(image_gallery)
    .bind(venue_place_id)
    .bind(venue_latitude)
    .bind(venue_longitude)
    .fetch_one(&mut **tx)
    .await
}

pub async fn update_event(
    pool: &PgPool,
    event_id: Uuid,
    organizer_id: Uuid,
    title: &str,
    description: Option<&str>,
    category: &str,
    venue_id: Option<Uuid>,
    venue_name: &str,
    venue_address: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    venue_template_id: Option<Uuid>,
    seating_mode: Option<SeatingMode>,
    image_url: Option<&str>,
    image_gallery: &Json<Vec<String>>,
    venue_place_id: Option<&str>,
    venue_latitude: Option<f64>,
    venue_longitude: Option<f64>,
) -> Result<Option<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        WITH updated AS (
            UPDATE events
            SET title = $3,
                description = $4,
                category = $5,
                venue_id = $6,
                venue_name = $7,
                venue_address = $8,
                start_time = $9,
                venue_template_id = $10,
                seating_mode = $11::text::seating_mode,
                image_url = $12,
                image_gallery = $13,
                venue_place_id = $14,
                venue_latitude = $15,
                venue_longitude = $16
            WHERE id = $1 AND organizer_id = $2
            RETURNING *
        )
        SELECT
            updated.id,
            updated.organizer_id,
            updated.title,
            updated.description,
            updated.category,
            updated.venue_name,
            updated.venue_address,
            updated.start_time,
            updated.status,
            updated.created_at,
            updated.venue_id,
            updated.venue_template_id,
            updated.seating_mode,
            updated.image_url,
            updated.image_gallery,
            updated.venue_place_id,
            updated.venue_latitude,
            updated.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity
        FROM updated
        LEFT JOIN event_venues v ON v.id = updated.venue_id
        "#,
    )
    .bind(event_id)
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_id)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .bind(venue_template_id)
    .bind(seating_mode)
    .bind(image_url)
    .bind(image_gallery)
    .bind(venue_place_id)
    .bind(venue_latitude)
    .bind(venue_longitude)
    .fetch_optional(pool)
    .await
}

pub async fn delete_event_transaction(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    organizer_id: Uuid,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM scan_logs
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM ticket_seats
        WHERE ticket_id IN (
            SELECT id FROM tickets WHERE event_id = $1
        )
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM tickets
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM order_items
        WHERE order_id IN (
            SELECT id FROM orders WHERE event_id = $1
        )
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM orders
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    let result = sqlx::query(
        r#"
        DELETE FROM events
        WHERE id = $1 AND organizer_id = $2
        "#,
    )
    .bind(event_id)
    .bind(organizer_id)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected())
}

pub async fn list_organizer_events(
    pool: &PgPool,
    organizer_id: Uuid,
) -> Result<Vec<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT
            e.id,
            e.organizer_id,
            e.title,
            e.description,
            e.category,
            e.venue_name,
            e.venue_address,
            e.start_time,
            e.status,
            e.created_at,
            e.venue_id,
            e.venue_template_id,
            e.seating_mode,
            e.image_url,
            e.image_gallery,
            e.venue_place_id,
            e.venue_latitude,
            e.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity
        FROM events e
        LEFT JOIN event_venues v ON v.id = e.venue_id
        WHERE e.organizer_id = $1
        ORDER BY e.created_at DESC
        "#,
    )
    .bind(organizer_id)
    .fetch_all(pool)
    .await
}

pub async fn get_organizer_dashboard_summary(
    pool: &PgPool,
    organizer_id: Uuid,
) -> Result<OrganizerDashboardSummaryResponse, sqlx::Error> {
    let events = sqlx::query_as::<_, OrganizerEventDashboardSummary>(
        r#"
        WITH order_stats AS (
            SELECT
                o.event_id,
                COUNT(DISTINCT o.id) FILTER (WHERE o.status = 'Completed')::bigint AS orders_completed,
                COALESCE(SUM(o.total_amount) FILTER (WHERE o.status = 'Completed'), 0)::float8 AS gross_revenue
            FROM orders o
            GROUP BY o.event_id
        ),
        ticket_stats AS (
            SELECT
                t.event_id,
                COUNT(*) FILTER (WHERE t.status <> 'Cancelled')::bigint AS tickets_sold
            FROM tickets t
            GROUP BY t.event_id
        ),
        scan_stats AS (
            SELECT
                sl.event_id,
                COUNT(*) FILTER (WHERE sl.result = 'Admitted')::bigint AS tickets_scanned,
                COUNT(*) FILTER (WHERE sl.result = 'Rejected')::bigint AS rejected_scans
            FROM scan_logs sl
            GROUP BY sl.event_id
        ),
        inventory_stats AS (
            SELECT
                esi.event_id,
                COUNT(*) FILTER (WHERE esi.status = 'Available')::bigint AS seats_available,
                COUNT(*) FILTER (WHERE esi.status = 'Held')::bigint AS seats_held,
                COUNT(*) FILTER (WHERE esi.status = 'Blocked')::bigint AS seats_blocked,
                COUNT(*)::bigint AS seats_total
            FROM event_seat_inventory esi
            GROUP BY esi.event_id
        )
        SELECT
            e.id AS event_id,
            e.title,
            e.category,
            e.venue_name,
            e.start_time,
            e.status,
            COALESCE(os.gross_revenue, 0)::float8 AS gross_revenue,
            COALESCE(os.orders_completed, 0)::bigint AS orders_completed,
            COALESCE(ts.tickets_sold, 0)::bigint AS tickets_sold,
            COALESCE(ss.tickets_scanned, 0)::bigint AS tickets_scanned,
            COALESCE(ss.rejected_scans, 0)::bigint AS rejected_scans,
            COALESCE(is1.seats_available, 0)::bigint AS seats_available,
            COALESCE(is1.seats_held, 0)::bigint AS seats_held,
            COALESCE(is1.seats_blocked, 0)::bigint AS seats_blocked,
            COALESCE(is1.seats_total, 0)::bigint AS seats_total
        FROM events e
        LEFT JOIN order_stats os ON os.event_id = e.id
        LEFT JOIN ticket_stats ts ON ts.event_id = e.id
        LEFT JOIN scan_stats ss ON ss.event_id = e.id
        LEFT JOIN inventory_stats is1 ON is1.event_id = e.id
        WHERE e.organizer_id = $1
        ORDER BY e.start_time ASC
        "#,
    )
    .bind(organizer_id)
    .fetch_all(pool)
    .await?;

    let total_events = events.len() as i64;
    let published_events = events
        .iter()
        .filter(|event| matches!(event.status, crate::db::models::EventStatus::Published))
        .count() as i64;
    let gross_revenue = events.iter().map(|event| event.gross_revenue).sum();
    let tickets_sold = events.iter().map(|event| event.tickets_sold).sum();
    let tickets_scanned = events.iter().map(|event| event.tickets_scanned).sum();
    let seats_available = events.iter().map(|event| event.seats_available).sum();
    let seats_held = events.iter().map(|event| event.seats_held).sum();
    let seats_blocked = events.iter().map(|event| event.seats_blocked).sum();
    let seats_total = events.iter().map(|event| event.seats_total).sum();

    Ok(OrganizerDashboardSummaryResponse {
        total_events,
        published_events,
        gross_revenue,
        tickets_sold,
        tickets_scanned,
        seats_available,
        seats_held,
        seats_blocked,
        seats_total,
        suspicious_alerts: 0,
        recent_alerts: Vec::new(),
        events,
    })
}

pub async fn list_assigned_gate_staff(
    pool: &PgPool,
    event_id: Uuid,
    organizer_id: Uuid,
) -> Result<Vec<GateStaffSummary>, sqlx::Error> {
    sqlx::query_as::<_, GateStaffSummary>(
        r#"
        SELECT u.id, u.email, u.name
        FROM gate_staff_event_assignments gsea
        JOIN users u ON u.id = gsea.gate_staff_id
        JOIN events e ON e.id = gsea.event_id
        WHERE gsea.event_id = $1
          AND e.organizer_id = $2
        ORDER BY u.name ASC, u.email ASC
        "#,
    )
    .bind(event_id)
    .bind(organizer_id)
    .fetch_all(pool)
    .await
}

pub async fn replace_gate_staff_assignments(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    organizer_id: Uuid,
    gate_staff_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let owns_event = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM events
        WHERE id = $1 AND organizer_id = $2
        "#,
    )
    .bind(event_id)
    .bind(organizer_id)
    .fetch_one(&mut **tx)
    .await?;

    if owns_event == 0 {
        return Err(sqlx::Error::RowNotFound);
    }

    sqlx::query(
        r#"
        DELETE FROM gate_staff_event_assignments
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    for gate_staff_id in gate_staff_ids {
        sqlx::query(
            r#"
            INSERT INTO gate_staff_event_assignments (event_id, gate_staff_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(event_id)
        .bind(gate_staff_id)
        .bind(organizer_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn list_assigned_events_for_gate_staff(
    pool: &PgPool,
    gate_staff_id: Uuid,
) -> Result<Vec<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT
               e.id,
               e.organizer_id,
               e.title,
               e.description,
               e.category,
               e.venue_name,
               e.venue_address,
               e.start_time,
               e.status,
               e.created_at,
               e.venue_id,
               e.venue_template_id,
               e.seating_mode,
               e.image_url,
               e.image_gallery,
               e.venue_place_id,
               e.venue_latitude,
               e.venue_longitude,
               v.locality AS venue_locality,
               v.city AS venue_city,
               v.state AS venue_state,
               v.pincode AS venue_pincode,
               v.country AS venue_country,
               v.landmark AS venue_landmark,
               v.capacity AS venue_capacity
        FROM gate_staff_event_assignments gsea
        JOIN events e ON e.id = gsea.event_id
        LEFT JOIN event_venues v ON v.id = e.venue_id
        WHERE gsea.gate_staff_id = $1
        ORDER BY e.start_time ASC
        "#,
    )
    .bind(gate_staff_id)
    .fetch_all(pool)
    .await
}

pub async fn cancel_event_transaction(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    organizer_id: Uuid,
) -> Result<Option<Event>, sqlx::Error> {
    let event = sqlx::query_as::<_, Event>(
        r#"
        WITH updated AS (
            UPDATE events
            SET status = 'Cancelled'
            WHERE id = $1
              AND organizer_id = $2
              AND status <> 'Cancelled'
            RETURNING *
        )
        SELECT
            updated.id,
            updated.organizer_id,
            updated.title,
            updated.description,
            updated.category,
            updated.venue_name,
            updated.venue_address,
            updated.start_time,
            updated.status,
            updated.created_at,
            updated.venue_id,
            updated.venue_template_id,
            updated.seating_mode,
            updated.image_url,
            updated.image_gallery,
            updated.venue_place_id,
            updated.venue_latitude,
            updated.venue_longitude,
            v.locality AS venue_locality,
            v.city AS venue_city,
            v.state AS venue_state,
            v.pincode AS venue_pincode,
            v.country AS venue_country,
            v.landmark AS venue_landmark,
            v.capacity AS venue_capacity
        FROM updated
        LEFT JOIN event_venues v ON v.id = updated.venue_id
        "#,
    )
    .bind(event_id)
    .bind(organizer_id)
    .fetch_optional(&mut **tx)
    .await?;

    if event.is_none() {
        return Ok(None);
    }

    sqlx::query(
        r#"
        DELETE FROM seat_holds
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE event_seat_inventory
        SET status = 'Blocked'::seat_status
        WHERE event_id = $1
          AND status IN ('Held'::seat_status, 'Sold'::seat_status)
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE tickets
        SET status = 'Cancelled',
            used_at = NULL
        WHERE event_id = $1
          AND status = 'Valid'
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE orders
        SET status = CASE
                WHEN status = 'Completed' THEN 'Refunded'
                WHEN status = 'Refunded' THEN status
                ELSE 'Cancelled'
            END,
            failure_reason = COALESCE(failure_reason, 'Event cancelled by organizer')
        WHERE event_id = $1
          AND status <> 'Refunded'
        "#,
    )
    .bind(event_id)
    .execute(&mut **tx)
    .await?;

    Ok(event)
}

pub async fn get_event_pulse(
    pool: &PgPool,
    event_id: Uuid,
) -> Result<crate::db::models::EventPulseResponse, sqlx::Error> {
    let inventory_stats = sqlx::query(
        r#"
        WITH reserved AS (
            SELECT
                COUNT(*) FILTER (WHERE status = 'Sold')::bigint AS sold_count,
                COUNT(*) FILTER (WHERE status = 'Held')::bigint AS held_count,
                COUNT(*)::bigint AS total_count
            FROM event_seat_inventory
            WHERE event_id = $1
        ),
        ga_capacity AS (
            SELECT COALESCE(SUM(capacity), 0)::bigint AS total_count
            FROM event_ticket_tiers
            WHERE event_id = $1
        ),
        ga_held AS (
            SELECT COUNT(*)::bigint AS held_count
            FROM seat_holds
            WHERE event_id = $1
              AND ticket_tier_id IS NOT NULL
              AND expires_at > NOW()
        ),
        ga_sold AS (
            SELECT COALESCE(SUM(tt.quantity), 0)::bigint AS sold_count
            FROM ticket_tiers tt
            JOIN tickets t ON t.id = tt.ticket_id
            WHERE t.event_id = $1
              AND t.status <> 'Cancelled'
        )
        SELECT
            (reserved.sold_count + ga_sold.sold_count) AS sold_count,
            (reserved.held_count + ga_held.held_count) AS held_count,
            (reserved.total_count + ga_capacity.total_count) AS total_count
        FROM reserved, ga_capacity, ga_held, ga_sold
        "#,
    )
    .bind(event_id)
    .fetch_one(pool)
    .await?;

    let recently_sold = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tickets
        WHERE event_id = $1 AND created_at > NOW() - INTERVAL '1 day'
        "#,
    )
    .bind(event_id)
    .fetch_one(pool)
    .await?;

    let total = inventory_stats
        .try_get::<Option<i64>, _>("total_count")?
        .unwrap_or(0);
    let sold = inventory_stats
        .try_get::<Option<i64>, _>("sold_count")?
        .unwrap_or(0);
    let held = inventory_stats
        .try_get::<Option<i64>, _>("held_count")?
        .unwrap_or(0);

    let sold_percentage = if total > 0 {
        (sold as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Calculate viewers based on real activity (holds + recent sales)
    let real_activity = held + recently_sold;

    // We know at least 1 person is viewing because this endpoint was just called!
    let active_viewers = std::cmp::max(1, real_activity);

    // Simplified section pulse for MVP: just general status
    let mut sections = Vec::new();
    if total > 0 {
        let status = if sold_percentage > 95.0 {
            "Sold Out"
        } else if sold_percentage > 80.0 {
            "Fast Filling"
        } else {
            "Available"
        };
        sections.push(crate::db::models::SectionPulse {
            section_name: "General".to_string(),
            status: status.to_string(),
        });
    }

    Ok(crate::db::models::EventPulseResponse {
        active_viewers,
        recently_sold,
        total_capacity: total,
        sold_percentage,
        sections,
    })
}
