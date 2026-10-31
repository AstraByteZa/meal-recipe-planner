-- Sync groundwork: UUID primary keys, ownership and soft-delete columns on every synced table,
-- and the outbox/sync-state tables. Existing rows are copied with fresh ids.
--
-- Old tables are copied to temp tables, then the children are dropped before their parents, so
-- no FK cascade can delete live data. Only one device has pre-release data, so the random
-- ids on migrated rows are fine; new rows get deterministic ids in the repo layer.

-- Random RFC 4122 v4 UUID as text. Re-evaluated per row when used inside INSERT ... SELECT.
-- Repeated inline below because SQLite has no user functions here.

CREATE TEMP TABLE old_tracks AS SELECT * FROM main.tracks;
CREATE TEMP TABLE old_recipes AS SELECT * FROM main.recipes;
CREATE TEMP TABLE old_tags AS SELECT * FROM main.tags;
CREATE TEMP TABLE old_recipe_tags AS SELECT * FROM main.recipe_tags;
CREATE TEMP TABLE old_schedule_entries AS SELECT * FROM main.schedule_entries;
CREATE TEMP TABLE old_settings AS SELECT * FROM main.settings;
CREATE TEMP TABLE old_household_diets AS SELECT * FROM main.household_diets;
CREATE TEMP TABLE old_config AS SELECT * FROM main.config;

CREATE TEMP TABLE map_recipe (old_id INTEGER PRIMARY KEY, new_id TEXT NOT NULL);
INSERT INTO map_recipe
SELECT id,
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6)))
FROM main.recipes;

CREATE TEMP TABLE map_tag (old_id INTEGER PRIMARY KEY, new_id TEXT NOT NULL);
INSERT INTO map_tag
SELECT id,
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6)))
FROM main.tags;

DROP TABLE main.recipe_tags;
DROP TABLE main.schedule_entries;
DROP TABLE main.recipes;
DROP TABLE main.tags;
DROP TABLE main.tracks;
DROP TABLE main.settings;
DROP TABLE main.household_diets;
DROP TABLE main.config;

CREATE TABLE tracks (
  id TEXT PRIMARY KEY,
  key TEXT NOT NULL UNIQUE,
  label TEXT NOT NULL,
  color TEXT NOT NULL,
  position INTEGER NOT NULL,
  default_time TEXT CHECK (default_time IS NULL OR default_time GLOB '[0-2][0-9]:[0-5][0-9]'),
  visible INTEGER NOT NULL DEFAULT 1 CHECK (visible IN (0, 1)),
  repeats_on TEXT NOT NULL DEFAULT '1111111'
    CHECK (length(repeats_on) = 7 AND repeats_on NOT GLOB '*[^01]*'),
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);
-- Positions only need to be unique among live tracks; removed tracks keep their old position.
CREATE UNIQUE INDEX tracks_live_position ON tracks(position) WHERE deleted_at IS NULL;

CREATE TABLE tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE recipes (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT NOT NULL,
  source_domain TEXT NOT NULL,
  minutes INTEGER CHECK (minutes IS NULL OR minutes > 0),
  rating INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 5),
  image_path TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  servings INTEGER CHECK (servings IS NULL OR servings > 0),
  notes TEXT,
  usual_track_id TEXT REFERENCES tracks(id) ON DELETE SET NULL,
  ingredients TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(ingredients)),
  instructions TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(instructions)),
  image_url TEXT,
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  deleted_at TEXT
);

CREATE TABLE recipe_tags (
  id TEXT PRIMARY KEY,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT,
  UNIQUE (recipe_id, tag_id)
);

CREATE TABLE schedule_entries (
  id TEXT PRIMARY KEY,
  date TEXT NOT NULL CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT,
  UNIQUE (date, track_id)
);

CREATE INDEX schedule_entries_recipe_id ON schedule_entries(recipe_id);

-- Single household row. The id is a fixed constant, so the repo can address it without a lookup.
CREATE TABLE settings (
  id TEXT PRIMARY KEY,
  household_size INTEGER NOT NULL DEFAULT 2 CHECK (household_size BETWEEN 1 AND 99),
  week_starts_on TEXT NOT NULL DEFAULT 'monday' CHECK (week_starts_on IN ('monday', 'sunday')),
  -- Device flag, not part of the synced payload.
  onboarded INTEGER NOT NULL DEFAULT 0 CHECK (onboarded IN (0, 1)),
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE household_diets (
  id TEXT PRIMARY KEY,
  diet TEXT NOT NULL UNIQUE CHECK (diet IN ('vegetarian', 'vegan', 'pescatarian', 'gluten-free', 'dairy-free', 'nut-free')),
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE config (
  id TEXT PRIMARY KEY,
  key TEXT NOT NULL UNIQUE CHECK (key GLOB '[a-z]*' AND length(key) <= 64),
  value TEXT NOT NULL CHECK (json_valid(value)),
  owner_id TEXT NOT NULL DEFAULT 'local',
  family_id TEXT,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

-- Sync state (server URL, token, last pulled seq, active family). Filled in by the sync worker.
CREATE TABLE sync_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Local mutations waiting to be pushed. A pending row per entity is coalesced by the partial index.
CREATE TABLE outbox_queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  operation TEXT NOT NULL CHECK (operation IN ('UPSERT', 'DELETE')),
  payload TEXT NOT NULL,
  client_timestamp TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'syncing', 'failed'))
);
CREATE UNIQUE INDEX outbox_queue_pending_entity ON outbox_queue(entity_type, entity_id)
  WHERE status = 'pending';

-- Seeded tracks keep the same ids on every device, so two devices never create duplicates.
INSERT INTO tracks (id, key, label, color, position, default_time, visible, repeats_on, updated_at)
SELECT
    CASE key
        WHEN 'breakfast' THEN '00000000-0000-4000-8000-000000000001'
        WHEN 'breakfast-kids' THEN '00000000-0000-4000-8000-000000000002'
        WHEN 'lunch' THEN '00000000-0000-4000-8000-000000000003'
        WHEN 'dinner' THEN '00000000-0000-4000-8000-000000000004'
        ELSE lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
            || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6)))
    END,
    key, label, color, position, default_time, visible, repeats_on,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_tracks;

INSERT INTO tags (id, name, updated_at)
SELECT m.new_id, t.name, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_tags t JOIN map_tag m ON m.old_id = t.id;

INSERT INTO recipes (id, name, url, source_domain, minutes, rating, image_path, created_at, updated_at,
                     servings, notes, usual_track_id, ingredients, instructions, image_url)
SELECT m.new_id, r.name, r.url, r.source_domain, r.minutes, r.rating, r.image_path, r.created_at, r.updated_at,
       r.servings, r.notes, (SELECT t.id FROM tracks t WHERE t.key = r.usual_track_key),
       r.ingredients, r.instructions, r.image_url
FROM old_recipes r JOIN map_recipe m ON m.old_id = r.id;

INSERT INTO recipe_tags (id, recipe_id, tag_id, updated_at)
SELECT
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6))),
    mr.new_id, mt.new_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_recipe_tags rt
JOIN map_recipe mr ON mr.old_id = rt.recipe_id
JOIN map_tag mt ON mt.old_id = rt.tag_id;

INSERT INTO schedule_entries (id, date, track_id, recipe_id, updated_at)
SELECT
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6))),
    se.date, t.id, mr.new_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_schedule_entries se
JOIN tracks t ON t.key = se.track_key
JOIN map_recipe mr ON mr.old_id = se.recipe_id;

INSERT INTO settings (id, household_size, week_starts_on, onboarded, updated_at)
SELECT '00000000-0000-4000-8000-000000000010', household_size, week_starts_on, onboarded,
       strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_settings;

INSERT INTO household_diets (id, diet, updated_at)
SELECT
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6))),
    diet, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_household_diets;

INSERT INTO config (id, key, value, updated_at)
SELECT
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))), 2) || '-'
    || substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6))),
    key, value, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM old_config;

DROP TABLE temp.map_recipe;
DROP TABLE temp.map_tag;
DROP TABLE temp.old_tracks;
DROP TABLE temp.old_recipes;
DROP TABLE temp.old_tags;
DROP TABLE temp.old_recipe_tags;
DROP TABLE temp.old_schedule_entries;
DROP TABLE temp.old_settings;
DROP TABLE temp.old_household_diets;
DROP TABLE temp.old_config;
