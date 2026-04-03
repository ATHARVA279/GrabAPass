ALTER TABLE events
    ADD COLUMN IF NOT EXISTS venue_place_id TEXT,
    ADD COLUMN IF NOT EXISTS venue_latitude DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS venue_longitude DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS image_gallery JSONB NOT NULL DEFAULT '[]'::jsonb;

UPDATE events
SET image_gallery = CASE
    WHEN image_url IS NULL OR btrim(image_url) = '' THEN '[]'::jsonb
    ELSE jsonb_build_array(image_url)
END
WHERE image_gallery = '[]'::jsonb;
