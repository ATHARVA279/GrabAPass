-- Phase 8: Split & Pay (Group Checkout)
-- Run this manually: sqlx migrate run

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'split_status') THEN
        CREATE TYPE split_status AS ENUM ('Pending', 'Completed', 'Expired', 'Refunded');
    END IF;
END $$;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'split_type') THEN
        CREATE TYPE split_type AS ENUM ('Even', 'Custom');
    END IF;
END $$;

-- ─── split_sessions ────────────────────────────────────────────────────────
-- A master record tracking the overarching split checkout.
CREATE TABLE IF NOT EXISTS split_sessions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id      UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    total_amount  NUMERIC(10, 2) NOT NULL,
    split_type    split_type NOT NULL DEFAULT 'Even',
    status        split_status NOT NULL DEFAULT 'Pending',
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── split_shares ──────────────────────────────────────────────────────────
-- Individual payment shares representing one friend's portion of the bill.
CREATE TABLE IF NOT EXISTS split_shares (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    split_session_id UUID NOT NULL REFERENCES split_sessions(id) ON DELETE CASCADE,
    amount_due       NUMERIC(10, 2) NOT NULL,
    status           split_status NOT NULL DEFAULT 'Pending',
    guest_name       VARCHAR(100),
    guest_email      VARCHAR(150),
    payment_token    UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(), -- Used for the public URL (/split/:token)
    gateway_order_id VARCHAR(255),
    gateway_payment_id VARCHAR(255),
    paid_at          TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_split_shares_token ON split_shares(payment_token);
CREATE INDEX IF NOT EXISTS idx_split_sessions_order ON split_sessions(order_id);
