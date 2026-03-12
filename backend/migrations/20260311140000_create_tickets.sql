-- Phase 6: Tickets + QR Generation
-- Each purchased seat generates one ticket with a unique QR payload.

CREATE TABLE IF NOT EXISTS tickets (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id      UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    order_item_id UUID NOT NULL REFERENCES order_items(id) ON DELETE CASCADE,
    event_id      UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    seat_id       UUID NOT NULL REFERENCES venue_seats(id),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    qr_secret     VARCHAR(64) NOT NULL UNIQUE,
    status        VARCHAR(20) NOT NULL DEFAULT 'Valid',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at       TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_tickets_order ON tickets(order_id);
CREATE INDEX IF NOT EXISTS idx_tickets_user ON tickets(user_id);
CREATE INDEX IF NOT EXISTS idx_tickets_event ON tickets(event_id);
CREATE INDEX IF NOT EXISTS idx_tickets_qr ON tickets(qr_secret);
