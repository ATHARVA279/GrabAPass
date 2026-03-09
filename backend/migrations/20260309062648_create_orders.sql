-- ─── orders ──────────────────────────────────────────────────────────────────
-- Top-level entity for a completed checkout/purchase.

CREATE TABLE IF NOT EXISTS orders (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id      UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    total_amount  FLOAT8 NOT NULL,
    status        VARCHAR(50) NOT NULL DEFAULT 'Completed',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── order_items ─────────────────────────────────────────────────────────────
-- The individually purchased seats linked to an order.

CREATE TABLE IF NOT EXISTS order_items (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id      UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    seat_id       UUID NOT NULL REFERENCES venue_seats(id),
    price         FLOAT8 NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_orders_user ON orders(user_id);
CREATE INDEX IF NOT EXISTS idx_orders_event ON orders(event_id);
CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_order_items_seat ON order_items(seat_id);
