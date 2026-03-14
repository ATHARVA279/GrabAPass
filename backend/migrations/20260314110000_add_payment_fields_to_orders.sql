ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS subtotal_amount FLOAT8 NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS fee_amount FLOAT8 NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS currency VARCHAR(10) NOT NULL DEFAULT 'INR',
    ADD COLUMN IF NOT EXISTS gateway VARCHAR(50),
    ADD COLUMN IF NOT EXISTS gateway_order_id VARCHAR(255),
    ADD COLUMN IF NOT EXISTS gateway_payment_id VARCHAR(255),
    ADD COLUMN IF NOT EXISTS payment_signature VARCHAR(255),
    ADD COLUMN IF NOT EXISTS payment_verified_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS receipt VARCHAR(255),
    ADD COLUMN IF NOT EXISTS failure_reason TEXT;

UPDATE orders
SET subtotal_amount = total_amount
WHERE subtotal_amount = 0;

ALTER TABLE orders
    ALTER COLUMN status SET DEFAULT 'Pending';

CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_gateway_order_id
    ON orders(gateway_order_id)
    WHERE gateway_order_id IS NOT NULL;
