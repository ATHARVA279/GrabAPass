-- Drop existing tickets. It's safe since it's dev, but we can also migrate structure.
-- Set scan_logs ticket_id to NULL so we don't violate the constraint when re-adding it
UPDATE scan_logs SET ticket_id = NULL;

ALTER TABLE scan_logs DROP CONSTRAINT scan_logs_ticket_id_fkey;

DROP TABLE tickets CASCADE;

CREATE TABLE tickets (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id      UUID NOT NULL REFERENCES orders(id),
    event_id      UUID NOT NULL REFERENCES events(id),
    user_id       UUID NOT NULL REFERENCES users(id),
    qr_secret     VARCHAR(100) NOT NULL, -- The HMAC secret payload part
    status        VARCHAR(20) NOT NULL DEFAULT 'Valid', -- Valid, Used, Cancelled
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at       TIMESTAMPTZ
);

CREATE INDEX idx_tickets_user_id ON tickets(user_id);
CREATE INDEX idx_tickets_event_id ON tickets(event_id);
CREATE INDEX idx_tickets_order_id ON tickets(order_id);

CREATE TABLE ticket_seats (
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    seat_id UUID NOT NULL REFERENCES venue_seats(id),
    PRIMARY KEY (ticket_id, seat_id)
);

-- Re-add the Foreign Key constraint correctly
ALTER TABLE scan_logs 
  ADD CONSTRAINT scan_logs_ticket_id_fkey 
  FOREIGN KEY (ticket_id) REFERENCES tickets(id) ON DELETE SET NULL;
