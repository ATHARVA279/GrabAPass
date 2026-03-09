-- ─── seat_holds ──────────────────────────────────────────────────────────────
-- Temporary holds placed on seats by users during checkout.
-- If the hold expires before an order is placed, the seat returns to 'Available'.

CREATE TABLE IF NOT EXISTS seat_holds (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    seat_id     UUID NOT NULL REFERENCES venue_seats(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    -- A seat can only have one active hold per event
    UNIQUE (event_id, seat_id)
);

CREATE INDEX IF NOT EXISTS idx_seat_holds_event ON seat_holds(event_id);
CREATE INDEX IF NOT EXISTS idx_seat_holds_user ON seat_holds(user_id);
CREATE INDEX IF NOT EXISTS idx_seat_holds_expires ON seat_holds(expires_at);
