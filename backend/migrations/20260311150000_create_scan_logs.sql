-- Add migration script here
CREATE TABLE scan_logs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id   UUID REFERENCES tickets(id),
    event_id    UUID NOT NULL REFERENCES events(id),
    scanned_by  UUID NOT NULL REFERENCES users(id),
    result      VARCHAR(20) NOT NULL,
    reason      TEXT,
    scanned_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scan_logs_event_id ON scan_logs(event_id);
CREATE INDEX idx_scan_logs_scanned_by ON scan_logs(scanned_by);
