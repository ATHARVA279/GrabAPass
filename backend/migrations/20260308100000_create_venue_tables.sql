-- Phase 3: Venue Layout + Event Seat Rendering
-- Run this manually: sqlx migrate run

-- ─── Enum Types ──────────────────────────────────────────────────────────────

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'seating_mode') THEN
        CREATE TYPE seating_mode AS ENUM ('Reserved', 'GeneralAdmission', 'Mixed');
    END IF;
END $$;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'seat_status') THEN
        CREATE TYPE seat_status AS ENUM ('Available', 'Held', 'Sold', 'Blocked');
    END IF;
END $$;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'stage_orientation') THEN
        CREATE TYPE stage_orientation AS ENUM ('North', 'South', 'East', 'West');
    END IF;
END $$;

-- ─── venue_templates ─────────────────────────────────────────────────────────
-- A reusable, named venue blueprint created by an organizer.
-- It defines the physical layout: sections → rows → seats.

CREATE TABLE IF NOT EXISTS venue_templates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organizer_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    stage_label     VARCHAR(100) NOT NULL DEFAULT 'STAGE',
    orientation     stage_orientation NOT NULL DEFAULT 'North',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── venue_sections ──────────────────────────────────────────────────────────
-- A named zone inside a venue (e.g. "Floor", "Balcony", "VIP Left").

CREATE TABLE IF NOT EXISTS venue_sections (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    venue_template_id   UUID NOT NULL REFERENCES venue_templates(id) ON DELETE CASCADE,
    name                VARCHAR(100) NOT NULL,   -- e.g. "Floor A"
    display_order       INT NOT NULL DEFAULT 0,  -- render order top → bottom
    color_hex           VARCHAR(7) NOT NULL DEFAULT '#4A90D9'  -- category color hint
);

-- ─── venue_rows ──────────────────────────────────────────────────────────────
-- A row within a section. row_label is the human-readable prefix (A, B, C…).
-- seat_count defines how many seats are in the row (may differ between rows).

CREATE TABLE IF NOT EXISTS venue_rows (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    section_id  UUID NOT NULL REFERENCES venue_sections(id) ON DELETE CASCADE,
    row_label   VARCHAR(10) NOT NULL,    -- e.g. "A", "B", "10"
    seat_count  INT NOT NULL CHECK (seat_count > 0),
    display_order INT NOT NULL DEFAULT 0
);

-- ─── venue_seats ─────────────────────────────────────────────────────────────
-- Individual seat definitions inside a row.
-- seat_number is sequential 1..N and seat_label = row_label || seat_number (A1, B4…).
-- Optional metadata flags for accessible, aisle, VIP, companion, blocked_default.

CREATE TABLE IF NOT EXISTS venue_seats (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    row_id          UUID NOT NULL REFERENCES venue_rows(id) ON DELETE CASCADE,
    seat_number     INT NOT NULL,          -- 1-based position within the row
    seat_label      VARCHAR(20) NOT NULL,  -- e.g. "A1", "B4"
    is_accessible   BOOLEAN NOT NULL DEFAULT FALSE,
    is_aisle        BOOLEAN NOT NULL DEFAULT FALSE,
    is_vip          BOOLEAN NOT NULL DEFAULT FALSE,
    is_companion    BOOLEAN NOT NULL DEFAULT FALSE,
    blocked_default BOOLEAN NOT NULL DEFAULT FALSE  -- always blocked unless overridden
);

-- ─── event_seat_categories ───────────────────────────────────────────────────
-- Price/tier assignments for a section within a specific event.
-- An organizer maps a section → category name + price + color.

CREATE TABLE IF NOT EXISTS event_seat_categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    section_id  UUID NOT NULL REFERENCES venue_sections(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL,    -- e.g. "VIP", "Standard", "GA Floor"
    price       NUMERIC(10, 2) NOT NULL,
    color_hex   VARCHAR(7) NOT NULL DEFAULT '#4A90D9',
    UNIQUE (event_id, section_id)
);

-- ─── event_seat_inventory ────────────────────────────────────────────────────
-- Per-event, per-seat status record. Created when event seating is initialised.
-- Mirrors all seats from the venue template at event creation time.

CREATE TABLE IF NOT EXISTS event_seat_inventory (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    seat_id     UUID NOT NULL REFERENCES venue_seats(id) ON DELETE CASCADE,
    status      seat_status NOT NULL DEFAULT 'Available',
    UNIQUE (event_id, seat_id)
);

-- ─── Add venue_template_id + seating_mode to events ─────────────────────────
-- Allow null so existing events are not broken (they have no seating).

ALTER TABLE events
    ADD COLUMN IF NOT EXISTS venue_template_id UUID REFERENCES venue_templates(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS seating_mode      seating_mode;

-- ─── Indexes ─────────────────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_venue_sections_template ON venue_sections(venue_template_id);
CREATE INDEX IF NOT EXISTS idx_venue_rows_section      ON venue_rows(section_id);
CREATE INDEX IF NOT EXISTS idx_venue_seats_row         ON venue_seats(row_id);
CREATE INDEX IF NOT EXISTS idx_esc_event               ON event_seat_categories(event_id);
CREATE INDEX IF NOT EXISTS idx_esi_event               ON event_seat_inventory(event_id);
CREATE INDEX IF NOT EXISTS idx_esi_seat                ON event_seat_inventory(seat_id);
CREATE INDEX IF NOT EXISTS idx_events_venue_template   ON events(venue_template_id);
