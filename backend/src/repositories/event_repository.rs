use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::db::models::{Event, GateStaffSummary, OrganizerEventDashboardSummary, OrganizerDashboardSummaryResponse, SeatingMode};

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
            SELECT id, organizer_id, title, description, category, venue_name, venue_address,
                   start_time, status, created_at, venue_template_id, seating_mode
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
            SELECT id, organizer_id, title, description, category, venue_name, venue_address,
                   start_time, status, created_at, venue_template_id, seating_mode
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
            SELECT id, organizer_id, title, description, category, venue_name, venue_address,
                   start_time, status, created_at, venue_template_id, seating_mode
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
            SELECT id, organizer_id, title, description, category, venue_name, venue_address,
                   start_time, status, created_at, venue_template_id, seating_mode
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
        SELECT id, organizer_id, title, description, category, venue_name, venue_address,
               start_time, status, created_at, venue_template_id, seating_mode
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
    venue_template_id: Option<Uuid>,
    seating_mode: Option<SeatingMode>,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events
            (organizer_id, title, description, category, venue_name, venue_address,
             start_time, status, venue_template_id, seating_mode)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'Published', $8, $9::text::seating_mode)
        RETURNING id, organizer_id, title, description, category, venue_name, venue_address,
                  start_time, status, created_at, venue_template_id, seating_mode
        "#,
    )
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .bind(venue_template_id)
    .bind(seating_mode)
    .fetch_one(pool)
    .await
}

pub async fn create_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    organizer_id: Uuid,
    title: &str,
    description: Option<&str>,
    category: &str,
    venue_name: &str,
    venue_address: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    venue_template_id: Option<Uuid>,
    seating_mode: Option<SeatingMode>,
) -> Result<Event, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events
            (organizer_id, title, description, category, venue_name, venue_address,
             start_time, status, venue_template_id, seating_mode)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'Published', $8, $9::text::seating_mode)
        RETURNING id, organizer_id, title, description, category, venue_name, venue_address,
                  start_time, status, created_at, venue_template_id, seating_mode
        "#,
    )
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .bind(venue_template_id)
    .bind(seating_mode)
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
    venue_name: &str,
    venue_address: &str,
    start_time: chrono::DateTime<chrono::Utc>,
    seating_mode: Option<SeatingMode>,
) -> Result<Option<Event>, sqlx::Error> {
    sqlx::query_as::<_, Event>(
        r#"
        UPDATE events
        SET title = $3,
            description = $4,
            category = $5,
            venue_name = $6,
            venue_address = $7,
            start_time = $8,
            seating_mode = $9::text::seating_mode
        WHERE id = $1 AND organizer_id = $2
        RETURNING id, organizer_id, title, description, category, venue_name, venue_address,
                  start_time, status, created_at, venue_template_id, seating_mode
        "#,
    )
    .bind(event_id)
    .bind(organizer_id)
    .bind(title)
    .bind(description)
    .bind(category)
    .bind(venue_name)
    .bind(venue_address)
    .bind(start_time)
    .bind(seating_mode)
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
        SELECT id, organizer_id, title, description, category, venue_name, venue_address,
               start_time, status, created_at, venue_template_id, seating_mode
        FROM events
        WHERE organizer_id = $1
        ORDER BY created_at DESC
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
    let published_events = events.iter().filter(|event| matches!(event.status, crate::db::models::EventStatus::Published)).count() as i64;
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
        SELECT e.id, e.organizer_id, e.title, e.description, e.category, e.venue_name, e.venue_address,
               e.start_time, e.status, e.created_at, e.venue_template_id, e.seating_mode
        FROM gate_staff_event_assignments gsea
        JOIN events e ON e.id = gsea.event_id
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
        UPDATE events
        SET status = 'Cancelled'
        WHERE id = $1
          AND organizer_id = $2
          AND status <> 'Cancelled'
        RETURNING id, organizer_id, title, description, category, venue_name, venue_address,
                  start_time, status, created_at, venue_template_id, seating_mode
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
