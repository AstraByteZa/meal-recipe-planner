ALTER TABLE recipes ADD COLUMN servings INTEGER CHECK (servings IS NULL OR servings > 0);
ALTER TABLE recipes ADD COLUMN notes TEXT;
ALTER TABLE recipes ADD COLUMN usual_track_key TEXT REFERENCES tracks(key) ON DELETE SET NULL;
