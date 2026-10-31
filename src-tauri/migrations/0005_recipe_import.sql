-- Full recipe details from web import. `recipes.url` already holds the original link, so it is the source URL.
ALTER TABLE recipes ADD COLUMN ingredients TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(ingredients));
ALTER TABLE recipes ADD COLUMN instructions TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(instructions));
ALTER TABLE recipes ADD COLUMN image_url TEXT;
