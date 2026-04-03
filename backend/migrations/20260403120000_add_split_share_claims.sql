ALTER TABLE split_shares
    ADD COLUMN IF NOT EXISTS claimed_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS claimed_ticket_id UUID REFERENCES tickets(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS claimed_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS split_share_order_item_allocations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    split_share_id UUID NOT NULL REFERENCES split_shares(id) ON DELETE CASCADE,
    order_item_id UUID NOT NULL UNIQUE REFERENCES order_items(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_split_share_item_allocations_share
    ON split_share_order_item_allocations(split_share_id);
