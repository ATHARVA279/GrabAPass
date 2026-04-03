ALTER TABLE split_shares
    ADD COLUMN IF NOT EXISTS is_host_share BOOLEAN NOT NULL DEFAULT FALSE;

WITH ranked_shares AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY split_session_id
            ORDER BY created_at ASC, id ASC
        ) AS rn
    FROM split_shares
)
UPDATE split_shares s
SET is_host_share = (ranked_shares.rn = 1)
FROM ranked_shares
WHERE ranked_shares.id = s.id;
