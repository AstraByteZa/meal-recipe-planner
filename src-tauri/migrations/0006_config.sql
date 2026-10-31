-- App configuration the user changes, stored as key/value pairs. Each value is a JSON
-- scalar (boolean, number or string), so new settings need no schema change.
CREATE TABLE config (
  key TEXT PRIMARY KEY CHECK (key GLOB '[a-z]*' AND length(key) <= 64),
  value TEXT NOT NULL CHECK (json_valid(value))
);
