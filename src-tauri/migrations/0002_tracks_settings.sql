ALTER TABLE tracks ADD COLUMN default_time TEXT
  CHECK (default_time IS NULL OR default_time GLOB '[0-2][0-9]:[0-5][0-9]');
ALTER TABLE tracks ADD COLUMN visible INTEGER NOT NULL DEFAULT 1 CHECK (visible IN (0, 1));
-- One character per weekday, Monday first. "1" means the track applies that day.
ALTER TABLE tracks ADD COLUMN repeats_on TEXT NOT NULL DEFAULT '1111111'
  CHECK (length(repeats_on) = 7 AND repeats_on NOT GLOB '*[^01]*');

UPDATE tracks SET default_time = '07:30' WHERE key IN ('breakfast', 'breakfast-kids');
UPDATE tracks SET default_time = '12:30' WHERE key = 'lunch';
UPDATE tracks SET default_time = '18:30' WHERE key = 'dinner';
