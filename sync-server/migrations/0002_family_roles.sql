-- Family owners and single-use invite codes. The app is unreleased, so the family invite codes
-- from 0001 are dropped rather than converted. Owners issue new codes.

ALTER TABLE family_members
    ADD COLUMN role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'member'));

-- The earliest joiner of each family becomes its owner.
UPDATE family_members AS m
SET role = 'owner'
FROM (
    SELECT DISTINCT ON (family_id) family_id, user_id
    FROM family_members
    ORDER BY family_id, joined_at, user_id
) AS first
WHERE m.family_id = first.family_id AND m.user_id = first.user_id;

-- At most one owner per family. Transfers demote the old owner before promoting the new one.
CREATE UNIQUE INDEX family_one_owner ON family_members (family_id) WHERE role = 'owner';

CREATE TABLE family_invites (
    id UUID PRIMARY KEY,
    family_id UUID NOT NULL REFERENCES families (id) ON DELETE CASCADE,
    code TEXT NOT NULL UNIQUE,
    created_by UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_by UUID REFERENCES users (id) ON DELETE CASCADE,
    used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX family_invites_family ON family_invites (family_id);

ALTER TABLE families DROP COLUMN invite_code;
