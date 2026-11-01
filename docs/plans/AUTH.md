# Optional accounts: local first, sync after sign-in

Status: proposed, not implemented. This is the first layer under `FAMILY_OWNER.md` and
`FAMILY_IMPL.md`. Where they disagree on names, sign-in or the signed-out `/family` page, this
document wins.

## 1. Goal
The app starts local only and is fully usable without an account. Signing in or creating an
account is optional. Sync, families and invites run only while the user is signed in. The account
owns the display name, which settles open question 1 in `FAMILY_IMPL.md`.

## 2. Decisions
- **Sign out:** two choices. Plain sign out keeps the link to the account. "Sign out and return
  this device to local only" unlinks everything.
- **Server address:** a build-time default, pre-filled and hidden under an "Advanced" toggle.
- **Entry points:** the sidebar chip and account button, the Family page empty state, and a
  skippable line on the onboarding Welcome step. No new onboarding step.
- **Full-page screens:** `/sign-in` and `/join` exist as full pages (see `docs/designs/auth/`), in
  addition to the sync settings modal. Both share one form component.
- **Join is two steps:** "Check code" calls a non-consuming preview endpoint
  (`GET /family/invites/{code}`) that returns the family name, the inviter, and member count; the
  code is consumed only on "Accept and join".

## 3. What already works (no change)
- The worker idles while signed out (`account::session` returns `None` without a token).
- New rows get `owner_id = 'local'`. `sign_in` runs `adopt_local_rows`, which gives them to the
  account and queues them. Deterministic ids merge duplicate tracks and tags.
- `family_*` commands already fail with "Sign in to sync…" when signed out.
- The server already stores `display_name`: `register` accepts it, `PATCH /me` edits it, and
  `UserDto` returns it.

## 4. Account modes
| Mode | Meaning | Chip |
| --- | --- | --- |
| `local` | No token and no linked account (`sync_meta.user_id` unset) | "Local only" |
| `signed_out` | Linked account, no token | "Local only" (with a "Sign in" hint) |
| `signed_in` | Token present | The five sync states from the designs |

`SyncStatus` gains `mode` and `user: { name, email } | null`. The waiting count is hidden outside
`signed_in`.

## 5. What to add

### 5.1 Display name
- `Api::register` sends `display_name`. `sign_in` stores `sync_meta.display_name` from
  `AuthResponse.user`.
- The Create account form gets a "Your name" field (optional, max length as the server). Sign in
  has no name field, since the server returns it.
- New `Api::update_me` (`PATCH /me`) and command `sync_set_name`. The account menu has "Your name".
- Server: `MemberDto.name` falls back to the email's local part when `display_name` is empty.
  Avatar initial and sidebar name use the same value.

### 5.2 Local-only status
- `status().pending` counts the outbox even when nothing can sync. Compute `mode` first and report
  0 pending unless `signed_in`.
- Stop growing the outbox on a purely local device: `outbox::record` returns early while
  `sync_meta.user_id` is unset. `adopt_local_rows` already re-records every local row at sign-in,
  so nothing is lost. Test: edit offline, sign in, and every row is pushed.

### 5.3 Sign out
- **Sign out** (existing `logout`): clears the token, email and name display. Keeps `user_id`,
  `owner_id`, `family_id` and the queue, so the same account signs back in and carries on.
  Limitation to state in the dialog: another account cannot sign in on this device.
- **Return to local only** (new `account::reset_to_local`), one transaction:
  - `UPDATE <table> SET owner_id = 'local', family_id = NULL` for every synced table.
  - Clear the outbox, `last_seq`, `user_id`, `email`, `display_name`, `token`, and the
    `active_family_*` and `family_notice` meta.
  - The server keeps its copy. The device is then local, and signing in later adopts the rows again
    (LWW settles any difference).
  - Confirm dialog: "Your recipes and plan stay on this device. They stop syncing and leave any
    family. Your account and its server data are not deleted."
- If the user is a family member or owner, sign out does not leave the family on the server.
  Say so in the dialog.

### 5.4 Server address
- A default from a build-time constant (`VITE_SYNC_SERVER_URL`, or a Rust const exposed by a
  command, whichever fits the mobile builds). The field is pre-filled and hidden under "Advanced".
- A previously stored `server_url` wins over the default.

### 5.5 Expired or rejected token
- On `SyncError::Unauthorized` the worker stops but the app still looks signed in. Map it to
  `lastErrorKind = "unauthorized"` and a "Sign in again" chip that opens the modal with the email
  prefilled. Check the current path at `worker.rs:60` before changing it.

### 5.6 Gating in the UI
- `/family` signed out: an empty state, "Family sharing needs an account", with a button that opens
  the sync modal on Create account. Signed in without a family: Create and Join cards.
- The family-details query and all family mutations are enabled only in `signed_in` with a family.
- `AccountButton` while local shows "Sign in" instead of a name.
- Onboarding Welcome step: one skippable line, "Optional: sync across devices later", linking to the
  modal. It never blocks the flow.

### 5.7 Out of scope (the cost of first-class accounts)
Change password, forgot password (needs email), and delete account (interacts with owning a
family). List them as follow-ups.

## 6. Order of work
0. **This layer.** Sections 5.1 to 5.5 in `shared-core`, the server fallback, `src-tauri` and the
   status schema, with the tests below. `pnpm build` and `cargo test` pass.
1. **Family logic.** `FAMILY_OWNER.md` steps 1 to 3 plus the changes in `FAMILY_IMPL.md` section 4.
2. **UI.** Sidebar chip with the local state, `AccountButton`, the signed-out `/family` page, the
   dialogs, the modal restyle with the Name field and sign-out choices (`FAMILY_IMPL.md` phase B,
   plus 5.6).
3. **Optional and docs.** Mobile shell, CLAUDE.md Sync section (local mode, no outbox while
   unlinked, reset to local), `docs/deploy-dokploy.md`.

## 7. Doc updates after this lands
- `FAMILY_IMPL.md` section 3: item 1 resolved (display name from the account), item 2 uses the
  signed-out state above, and the status fields in section 4 include `mode` and `user`.

## 8. Tests
- Unit (`src-tauri`): `record` skipped while unlinked and not after sign-in; `adopt_local_rows`
  re-records everything; `reset_to_local` moves rows, clears the outbox and meta; status `mode` table
  (local, signed_out, signed_in) and pending hidden when not signed in; register stores the name.
- Server: register with a name and without (fallback in `MemberDto`).
- e2e (`SYNC_TEST_SERVER_URL`): local edits, register with a name, rows reach a second device, then
  return to local: rows stay, stop syncing, and a different account can now sign in.
- Frontend (Node Vitest): `chipState` for `local` and `signed_out`, status schema with `mode` and
  `user`.
- Manual (`pnpm tauri dev`, fresh data dir): use the app signed out, create an account with a name,
  sign out, sign back in, return to local, then sign in as another account.
