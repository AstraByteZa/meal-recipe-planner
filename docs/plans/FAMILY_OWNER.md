# Family owner role and invite system

Status: proposed, not implemented.

## Context
The sync engine is finished and pushed. Families work, but they have no administration. Today:
- Every family has one permanent `invite_code` stored on the `families` row. Every member sees it in `GET /family`, it never expires, and it can never be revoked, so a leaked code is a permanent hole.
- `family_members` has no role. Nobody can rename the family, remove a member, leave, or delete it.
- The client only learns about its family from its own `family_create` / `family_join` calls. If it is removed, it keeps a stale `active_family_id` and its pushes are silently rejected ("not a member") and dropped.
- A second device of an account that is already in a family cannot get the family without an invite code, because it never asks the server which families the account belongs to.

Decisions made with the user:
- **Invites:** owner-issued, single-use, expiring codes. Not email based. Members never see codes.
- **Leaving or removal:** the person keeps the family rows on their device as private data. Nothing is deleted.
- **Owner powers:** remove a member, rename the family, transfer ownership, delete the family.

Defaults chosen (change them here before implementing if needed):
- Two roles only: `owner` (exactly one per family) and `member`.
- Invites last 7 days, with at most 10 pending per family. The code looks like `K7M2-QX9P` (8 characters, no look-alikes, not case or dash sensitive).
- The owner cannot leave. They must transfer ownership first, or delete the family.
- Members see each other's display name, falling back to email.
- A device still has one active family. A signed-in device with no active family adopts the account's family automatically when the server lists exactly one.

## Server (`sync-server`)

### Migration `0002_family_roles.sql`
- `family_members.role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner','member'))`. Backfill: the earliest joiner of each family (the creator) becomes owner.
- `CREATE UNIQUE INDEX family_one_owner ON family_members (family_id) WHERE role = 'owner'`.
- New table `family_invites (id UUID PK, family_id FK ON DELETE CASCADE, code TEXT UNIQUE, created_by UUID FK, created_at, expires_at, used_by UUID NULL, used_at NULL, revoked_at NULL)`.
- `ALTER TABLE families DROP COLUMN invite_code`. The app is unreleased, so old codes are not carried over. Owners issue new ones.

### Wire types (`shared-core/src/wire.rs`)
Add `FamilyRole`, `FamilyDto { id, name, role }`, `MemberDto { user_id, name, role, joined_at }`, `InviteDto { id, code, created_at, expires_at }`, `FamilyDetails { family, members, invites }` (`invites` is empty unless the caller is the owner). Server row structs map into these (shared-core stays free of sqlx).

Add `SseEvent::MEMBERSHIP_CHANGED` and a `membership_changed()` constructor. Give `seq` a serde default of 0, so the event still parses on clients that only know `op_ready`.

### Endpoints (`family.rs` split into `family.rs`, `invite.rs`, `member.rs`)
Shared helpers `require_member` and `require_owner`. A non-member gets 404, so family ids do not leak. A member who is not the owner gets 403.

| Route | Who | Behaviour |
| --- | --- | --- |
| `GET /family` | member | List with `role` (no code) |
| `GET /family/{id}` | member | `FamilyDetails` |
| `PATCH /family/{id}` | owner | Rename |
| `DELETE /family/{id}` | owner | Collect member ids, delete (FKs cascade entities, op_log, invites, members), then notify |
| `POST /family/{id}/invites` | owner | Create a code, enforce the pending cap |
| `DELETE /family/{id}/invites/{invite}` | owner | Revoke |
| `POST /family/join` | any user | Atomic `UPDATE family_invites SET used_by, used_at WHERE code AND unused AND unrevoked AND unexpired RETURNING family_id`, then insert the member, in one transaction. An existing member does not burn the code. All failures give one generic "invite is not valid or has expired" |
| `DELETE /family/{id}/members/{user}` | owner, or the user themself | Remove or leave. The owner cannot be removed or leave. |
| `POST /family/{id}/owner` | owner | Transfer in one transaction, demoting the old owner first so the unique index holds |

`POST /family/create` makes the creator the owner. Failed joins are throttled per user (about 10 per 15 minutes, in memory like the hub), which needs a new `ApiError::TooManyRequests` (429). Invite codes come from the existing `rand_core` dependency.

### Live notice (`hub.rs`, `sync.rs`)
The hub carries `Notice { Ops(i64), Membership }` instead of a bare `i64`. Every membership change (join, remove, leave, transfer, delete) sends `Membership` on each affected user's own channel, which `stream` already subscribes to. The stream turns it into a `membership_changed` event. Access control itself needs no change: push and pull already re-check `family_ids` on every request, so removal takes effect on the next call. The stream carries sequence numbers only, so a stale subscription leaks nothing.

## Client (`src-tauri`)

- `client.rs`: `Api` gains `families`, `family_details`, `create_invite`, `revoke_invite`, `rename_family`, `remove_member`, `transfer_ownership`, `delete_family`. `FamilyDto` drops `invite_code` and gains `role`. `EventStream` surfaces `membership_changed`.
- `account.rs`:
  - `detach_family(conn, user_id, family_id)`: for every synced table, `SET family_id = NULL, owner_id = <me> WHERE family_id = ?`, then `record` each row (this replaces any queued payload that still names the family). Clear the `active_family_*` meta. The rows are then pushed as the person's private copy, so nothing is lost.
  - `reconcile_family(pool, api)`: fetch `GET /family`. If the active family is listed, refresh its name and role. If it is missing, `detach_family`. If there is no active family and exactly one is listed, activate it with the pull cursor rewound to 0, as `family_join` does. Returns whether anything changed. The decision is a small pure function so it can be unit tested.
  - `FamilyInfo` drops `invite_code` and gains `role`. The stale `active_family_invite_code` meta is cleared.
- `worker.rs`: run `reconcile_family` once at session start, before the first sync, and again on `membership_changed`. If it changed anything, call `handle.restart()`, so the stream resubscribes. Running it in the session task keeps it serial with push and pull.
- `commands.rs` (thin wrappers): `family_details`, `family_invite_create`, `family_invite_revoke`, `family_rename`, `family_member_remove` (leaving is removing yourself, and detaches locally at once), `family_transfer`, `family_delete` (detaches locally at once). Register them in `lib.rs`.

## Frontend (`src/features/sync`)
- `schema.ts`: `family` becomes `{ id, name, role }`. Add schemas for `FamilyDetails`, member and invite.
- `queries.ts` / `mutations.ts`: a family-details query (key under `["sync"]`) and mutation hooks for the new commands, following `useSyncCommand`.
- New `components/FamilyPanel.tsx` (with a `.module.css`), used by `SyncSettingsModal`'s `FamilySection`:
  - Members list with role badges.
  - Owner only: create invite (shows the code with a copy button and its expiry), pending invites with revoke, rename, a per-member menu (remove, make owner), and delete (confirm by typing the family name, via the modals manager).
  - Member only: leave.
  - The join form stays for devices with no family. Error text comes from the generic server message.

## Delivery order (each step compiles and passes tests)
1. `shared-core` wire types and the SSE event, with serde tests.
2. Server migration, endpoints, hub notice and throttle, with `tests/family.rs`.
3. Client API, reconcile and detach, worker hook, commands, and unit tests.
4. Frontend panel, schema tests, `pnpm build`.
5. Docs: CLAUDE.md Sync section (roles, invites, reconcile, detach) and the family step in `docs/deploy-dokploy.md`.

## Verification
- `cargo test` with `TEST_DATABASE_URL`. New server tests (`sync-server/tests/family.rs`):
  - A member cannot create invites, and a non-member gets 404.
  - A code is single-use, and expired, revoked and reused codes all fail with the same message.
  - Rename is owner-only.
  - Removing a member makes their push for that family rejected and their pull exclude family rows.
  - The owner cannot leave, and transfer swaps roles.
  - Deleting the family cascades its rows, and the former members still see their private rows.
  - The join throttle triggers.
  - `membership_changed` reaches the affected user's stream.
  - A migration test: apply 0001, insert a family with two members, apply 0002, and assert the creator is owner.
- Client unit tests: `detach_family` (rows become private, owner set, outbox re-recorded, meta cleared) and the reconcile decision table.
- Extend `src-tauri/src/sync/e2e.rs` with `SYNC_TEST_SERVER_URL`:
  - The owner invites, the second account joins with the code, and the code then fails.
  - The owner removes the member. After reconcile, the member's family recipes are still present, private, and queued, and the stranger still cannot see them.
  - A second device of the owner adopts the family without a code.
- `pnpm build && pnpm test`. Then a manual `pnpm tauri dev` pass through the panel with two data dirs.
