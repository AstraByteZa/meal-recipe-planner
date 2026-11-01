# Family owner and invites: implementation plan (design-aware)

Status: proposed, not implemented. This builds on `FAMILY_OWNER.md` (the server, client and
sync behaviour) and replaces it wherever the two differ. The new designs are in
`docs/designs/family/`, plus the refreshed `Schedule@1x.png` and `Recipe links@1x.png`.

## 1. What the designs confirm

These match `FAMILY_OWNER.md`, so the behaviour there stands:
- Invites are owner-only, single-use, expire in 7 days, and are listed with a Revoke action.
  A newly made code appears in a large monospace box with a Copy button.
- Code format `XXXX-XXXX`. A bad code shows one generic message: "That invite code is not valid
  or has expired".
- One owner per family. The owner cannot leave ("make someone else the owner first, or delete the
  family for everyone"). Transfer reads "You will become a regular member".
- Removing a member, leaving, and deleting all say the rows stay on the device as private data.
  Delete needs the family name typed to enable the red button.
- Members see each other's name and email, with `Owner` / `Member` badges and a `you` marker.
- A banner tells a removed user: "You were removed from {family}. Your recipes are kept on this
  device."

## 2. What the designs change or add

| Area | Design | Effect on the plan |
| --- | --- | --- |
| Where family lives | A **Family page** (`/family`, nav item, eyebrow "Settings") | The member and owner panels are a route, not a section of the sync modal. The modal keeps sign-in, create and join only. |
| Member data | Name **and email** for every member | `MemberDto` gains `email`. The list shows both to all members. |
| Display name | "Alex Morgan" in the list, the avatar and the sidebar | Nothing in the app collects a name. See open question 1. |
| Sidebar footer | Sync chip (5 states) plus an account button (avatar, name, family name, chevron menu) | Replaces `SyncButton`. `UserButton` is unused template code with a hard-coded mock user, so it is rewritten. |
| Sidebar top | No search box | Remove the placeholder Ctrl+K search from `NavbarSearch`. |
| Mobile | Bottom tab bar (Schedule, Recipe links, Family), icon-only sync chip top right | New app shell. Today `DashboardLayout` is a fixed 230px sidebar with no mobile handling. |
| Removed banner | Persistent, with Dismiss | The client must remember it. New `family_notice` state (section 4). |
| Sync chip states | Sync off, Syncing (n changes), Synced (relative time), Failed (Retry), N waiting (badge, "will sync when online") | The status needs an error kind and an offline flag. |
| Sign-in modal | Server, email, password with Show, buttons "Register" and "Sign in" | Current modal uses a segmented control. Restyle to match. |
| Signed-in modal | Account card with Sign out. No sync-now or status text | Status moves to the sidebar chip, and Retry moves to the error chip. |

## 3. Gaps: not designed, so I need a decision or a default

1. **Display name source.** Default: the server computes `name` as `display_name`, falling back to
   the email's local part. Add a "Your name" field later (`PATCH /me` already exists).
2. **`/family` when signed out or in no family.** Default: an empty-state card with a "Set up
   sync" button that opens the sync modal (which already has create and join).
3. **Rename dialog.** Default: a standard modal with one text field, in the same style as the
   other dialogs.
4. **Throttled joins (429)** and the **10-pending-invites cap.** Default: inline error text under
   the code field, and the Invite button disabled with a hint.
5. **Family deleted by the owner.** Default: the same banner with "{family} was deleted by its
   owner. Your recipes are kept on this device."
6. **Account menu contents** (the chevron). Default: Sync now, Sync settings, Sign out.
7. **Newly made invite vs the pending list.** The design shows `K7M2-QX9P` in the big box but not
   in the list. Default: the box shows the latest code, and the list shows every other pending code.
8. **Mobile for Schedule and Recipe links.** There are mobile designs for Family and Cook only.
   The tab bar would sit over desktop layouts. Decide whether the shell ships now or after those
   pages get mobile layouts (section 6, phase C).

Outside this feature but visible in the new Schedule design: a **Snack** row with compact cells
and a dashed **Add track** row under the grid. The app has the Tracks button but not the Add track
row. Track this separately.

## 4. Changes to the data and API contracts

On top of `FAMILY_OWNER.md`:
- `shared-core` `MemberDto { user_id, name, email, role, joined_at }`.
- `SyncStatus.family` becomes `{ id, name, role }` and the status gains `lastErrorKind`
  (`"unreachable" | "unauthorized" | "other" | null`), `user: { name, email } | null`, and
  `notice: { kind: "removed" | "deleted", familyName } | null`.
- Client: `detach_family` (from a removal or deletion, not from the user's own leave) stores
  `sync_meta.family_notice`. New command `sync_dismiss_notice` clears it.
- Client: `SyncError` kinds map to `lastErrorKind`. Offline means `Unreachable` with changes
  waiting, which drives "will sync when online".
- `FamilyDetails` is fetched live by the Family page (query key `["sync", "family", id]`) and
  refetched on `sync://changed` and on `membership_changed`.

## 5. Frontend architecture

New and changed files, under `src/features/sync/` unless stated:
- `routes`: add `/family` in `src/router.tsx` (manually, per CLAUDE.md), with a `FamilyView`.
  Sidebar and tab bar link to it. It loads through the route loader, as the other pages do.
- `views/FamilyView.tsx`: header card (avatar initial, name, member count and role, Rename for the
  owner), members card, then the owner column (invite card, danger zone) or the member card
  (leave). Two columns for the owner at desktop width, one column below the `sm` breakpoint.
- `components/`: `MemberRow`, `InviteCard` (code box and Copy), `PendingInvites`,
  `DangerZone`, `RemovedBanner`, `SyncChip` (replaces `SyncButton`), `AccountButton`
  (replaces `UserButton`). Each has a `*.module.css` and uses theme colours and breakpoints,
  not hex values.
- `modals/`: `ConfirmRemoveMember`, `ConfirmTransfer`, `ConfirmDelete` (typed name), `ConfirmLeave`,
  `RenameFamily`, all registered in `src/features/planner/modals/registry.ts`. `SyncSettingsModal`
  is restyled to match, and its family section is reduced to create and join.
- Pure logic so it can be tested without a DOM (the Vitest setup is Node-only): `chipState(status)`
  (icon, title, subtitle, badge), `formatInviteCode` / `normalizeInviteCode`, `expiresIn(date)`,
  and the zod schemas. `src/lib/dayjs.ts` needs the `relativeTime` plugin registered for "2 min ago".
- `src/components/NavbarSearch`: remove the search input, add Family, swap the footer.
  `src/components/Layout/DashboardLayout.tsx`: add the mobile shell (phase C).

## 6. Delivery order (each phase compiles and passes tests)

**Phase A: server and client logic.** `FAMILY_OWNER.md` delivery steps 1 to 3, with the changes in
section 4: `MemberDto.email`, name fallback, `family_notice`, `sync_dismiss_notice`, status fields.
Verification as listed there, plus: a removed user's status carries the `removed` notice until
dismissed.

**Phase B: sidebar and Family page.** Remove search, add the nav item and route, `SyncChip` with
its five states, `AccountButton`, `FamilyView` with all members, invite and danger components, the
five dialogs, the removed banner, and the modal restyle. `pnpm build && pnpm test`.

**Phase C: mobile shell** (only if open question 8 says now). Bottom tab bar below the `sm`
breakpoint, sidebar hidden below it, compact header chip, and bottom padding for the bar. Set
`viewport-fit=cover` in `index.html` and use `env(safe-area-inset-bottom)`, because this also runs
on iOS and Android.

**Phase D: docs and assets.** Commit the design files (the old `schedule.png` and
`recipe_links.png` are deleted in the working tree and the new ones are named `Schedule@1x.png`
and `Recipe links@1x.png`). Fix the stale paths in CLAUDE.md ("Target design" line). Update the
CLAUDE.md Sync section and `docs/deploy-dokploy.md` as in `FAMILY_OWNER.md`.

## 7. Verification
- Backend and sync: as in `FAMILY_OWNER.md`.
- `pnpm test`: `chipState` for all five states, invite code formatting, `expiresIn`, the status
  and family-details zod schemas.
- `pnpm build` for the type check.
- Manual, with `pnpm tauri dev` and two data dirs against a local server: walk the owner flow
  (create family, invite, copy, join from the second instance, make owner, remove, delete), the
  member flow (leave), the invalid and throttled code states, the removed banner on the second
  instance, and offline (stop the server) to see the Failed and "waiting" chips. Compare each
  screen against `docs/designs/family/`.
