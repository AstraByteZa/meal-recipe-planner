-- Accounts and families.
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE families (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    invite_code TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE family_members (
    family_id UUID NOT NULL REFERENCES families (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (family_id, user_id)
);

CREATE INDEX family_members_user ON family_members (user_id);

-- Current state of every synced row. A row is private to its owner or shared with a family.
-- `scope_id` is the owner for private rows and the family for shared rows, so the same
-- deterministic id (for example a tag or the settings row) can exist once per scope.
CREATE TABLE entities (
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    owner_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id UUID REFERENCES families (id) ON DELETE CASCADE,
    scope_id UUID GENERATED ALWAYS AS (COALESCE(family_id, owner_id)) STORED,
    payload JSONB,
    updated_at TIMESTAMPTZ NOT NULL,
    updated_by_client TEXT NOT NULL,
    deleted_at TIMESTAMPTZ,
    PRIMARY KEY (scope_id, entity_type, entity_id)
);

-- Every accepted change, in commit order. Clients pull from a `seq` cursor.
CREATE TABLE op_log (
    seq BIGSERIAL PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    op TEXT NOT NULL CHECK (op IN ('UPSERT', 'DELETE')),
    owner_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id UUID REFERENCES families (id) ON DELETE CASCADE,
    client_id TEXT NOT NULL,
    client_timestamp TIMESTAMPTZ NOT NULL,
    payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX op_log_owner ON op_log (owner_id, seq) WHERE family_id IS NULL;
CREATE INDEX op_log_family ON op_log (family_id, seq) WHERE family_id IS NOT NULL;
