-- Phase 7: General Admission (GA) Ticket Tiers
-- Run this manually: sqlx migrate run

CREATE TABLE IF NOT EXISTS event_ticket_tiers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL,
    price       NUMERIC(10, 2) NOT NULL,
    capacity    INT NOT NULL CHECK (capacity > 0),
    color_hex   VARCHAR(7) NOT NULL DEFAULT '#4A90D9',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Alter seat_holds to support GA tiers instead of seats
ALTER TABLE seat_holds 
    DROP CONSTRAINT IF EXISTS seat_holds_event_id_seat_id_key;

ALTER TABLE seat_holds
    ADD COLUMN ticket_tier_id UUID REFERENCES event_ticket_tiers(id) ON DELETE CASCADE,
    ALTER COLUMN seat_id DROP NOT NULL,
    ADD CONSTRAINT check_hold_target CHECK (
        (seat_id IS NOT NULL AND ticket_tier_id IS NULL) OR 
        (seat_id IS NULL AND ticket_tier_id IS NOT NULL)
    );

-- Replace the UNIQUE constraint with a partial index only for reserved seats
CREATE UNIQUE INDEX IF NOT EXISTS idx_seat_holds_reserved ON seat_holds (event_id, seat_id) WHERE seat_id IS NOT NULL;

-- Alter order_items to support GA tiers instead of seats
ALTER TABLE order_items
    ADD COLUMN ticket_tier_id UUID REFERENCES event_ticket_tiers(id) ON DELETE CASCADE,
    ALTER COLUMN seat_id DROP NOT NULL,
    ADD CONSTRAINT check_order_item_target CHECK (
        (seat_id IS NOT NULL AND ticket_tier_id IS NULL) OR 
        (seat_id IS NULL AND ticket_tier_id IS NOT NULL)
    );

-- Add ticket_tiers for consolidated tickets
CREATE TABLE IF NOT EXISTS ticket_tiers (
    ticket_id      UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    ticket_tier_id UUID NOT NULL REFERENCES event_ticket_tiers(id) ON DELETE CASCADE,
    quantity       INT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (ticket_id, ticket_tier_id)
);
