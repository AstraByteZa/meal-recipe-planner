-- Sync worker support: track ordering and the starting timestamps of seeded rows.

-- Positions are ordered, not unique. Another device can reorder tracks, and pulled rows arrive
-- one at a time, so a unique index would reject states that are valid once the batch lands.
DROP INDEX tracks_live_position;
CREATE INDEX tracks_live_position ON tracks(position) WHERE deleted_at IS NULL;

-- Seeded rows (the four default tracks and the settings row) get the same fixed ids on every
-- device, but each device stamped them with its own install time. Give them the epoch so that
-- any real change from another device wins last-write-wins.
UPDATE tracks SET updated_at = '1970-01-01T00:00:00.000Z'
WHERE id IN (
    '00000000-0000-4000-8000-000000000001',
    '00000000-0000-4000-8000-000000000002',
    '00000000-0000-4000-8000-000000000003',
    '00000000-0000-4000-8000-000000000004'
);
UPDATE settings SET updated_at = '1970-01-01T00:00:00.000Z'
WHERE id = '00000000-0000-4000-8000-000000000010';
