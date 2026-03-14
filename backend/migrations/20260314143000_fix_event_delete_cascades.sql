ALTER TABLE scan_logs
    DROP CONSTRAINT IF EXISTS scan_logs_event_id_fkey;

ALTER TABLE scan_logs
    ADD CONSTRAINT scan_logs_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE tickets
    DROP CONSTRAINT IF EXISTS tickets_event_id_fkey;

ALTER TABLE tickets
    ADD CONSTRAINT tickets_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE tickets
    DROP CONSTRAINT IF EXISTS tickets_order_id_fkey;

ALTER TABLE tickets
    ADD CONSTRAINT tickets_order_id_fkey
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE;
