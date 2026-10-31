-- Household settings and onboarding state. The CHECK on id keeps this a single row.
CREATE TABLE settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  household_size INTEGER NOT NULL DEFAULT 2 CHECK (household_size BETWEEN 1 AND 99),
  week_starts_on TEXT NOT NULL DEFAULT 'monday' CHECK (week_starts_on IN ('monday', 'sunday')),
  onboarded INTEGER NOT NULL DEFAULT 0 CHECK (onboarded IN (0, 1))
);

INSERT INTO settings (id) VALUES (1);

CREATE TABLE household_diets (
  diet TEXT PRIMARY KEY CHECK (diet IN ('vegetarian', 'vegan', 'pescatarian', 'gluten-free', 'dairy-free', 'nut-free'))
);
