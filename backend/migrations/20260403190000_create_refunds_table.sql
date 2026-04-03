CREATE TABLE IF NOT EXISTS refunds (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    booking_id      UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    payment_id      VARCHAR(255),
    amount          NUMERIC(10, 2) NOT NULL DEFAULT 0,
    refund_status   VARCHAR(20) NOT NULL,
    refund_reason   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_refunds_booking_id ON refunds(booking_id, created_at DESC);
