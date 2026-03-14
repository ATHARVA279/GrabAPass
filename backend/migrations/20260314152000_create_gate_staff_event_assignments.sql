CREATE TABLE IF NOT EXISTS gate_staff_event_assignments (
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    gate_staff_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assigned_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, gate_staff_id)
);

CREATE INDEX IF NOT EXISTS idx_gate_staff_event_assignments_staff
    ON gate_staff_event_assignments(gate_staff_id, assigned_at DESC);
