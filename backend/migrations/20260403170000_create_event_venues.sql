CREATE TABLE IF NOT EXISTS event_venues (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_by  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    place_id    TEXT NOT NULL UNIQUE,
    latitude    DOUBLE PRECISION NOT NULL,
    longitude   DOUBLE PRECISION NOT NULL,
    address     TEXT NOT NULL,
    locality    VARCHAR(255) NOT NULL DEFAULT '',
    city        VARCHAR(255) NOT NULL DEFAULT '',
    state       VARCHAR(255) NOT NULL DEFAULT '',
    pincode     VARCHAR(32) NOT NULL DEFAULT '',
    country     VARCHAR(255) NOT NULL DEFAULT '',
    landmark    VARCHAR(255),
    capacity    INT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE events
    ADD COLUMN IF NOT EXISTS venue_id UUID REFERENCES event_venues(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_event_venues_created_by ON event_venues(created_by, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_event_venues_name_city ON event_venues(LOWER(name), LOWER(city), LOWER(state));
CREATE INDEX IF NOT EXISTS idx_event_venues_coordinates ON event_venues(latitude, longitude);
CREATE INDEX IF NOT EXISTS idx_events_venue_id ON events(venue_id);
