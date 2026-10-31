CREATE TABLE recipes (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT NOT NULL,
  source_domain TEXT NOT NULL,
  minutes INTEGER CHECK (minutes IS NULL OR minutes > 0),
  rating INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 5),
  image_path TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE tags (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE recipe_tags (
  recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (recipe_id, tag_id)
);

CREATE TABLE tracks (
  key TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  color TEXT NOT NULL,
  position INTEGER NOT NULL UNIQUE
);

CREATE TABLE schedule_entries (
  date TEXT NOT NULL CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  track_key TEXT NOT NULL REFERENCES tracks(key) ON DELETE CASCADE,
  recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  PRIMARY KEY (date, track_key)
);

CREATE INDEX schedule_entries_recipe_id ON schedule_entries(recipe_id);

INSERT INTO tracks (key, label, color, position) VALUES
  ('breakfast', 'Breakfast', 'yellow', 0),
  ('breakfast-kids', 'Breakfast Kids', 'yellow', 1),
  ('lunch', 'Lunch', 'myColor', 2),
  ('dinner', 'Dinner', 'blue', 3);
