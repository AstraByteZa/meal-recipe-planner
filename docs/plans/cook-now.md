# Cook now: amended plan

Status: approved direction, not started. Desktop first.

## Decisions

| # | Topic | Decision |
|---|---|---|
| 1 | Layout | Desktop first. Mobile is in TODO.md. |
| 2 | Duration parsing tests | Vitest, added as a dev dependency. |
| 3 | Oven chips (°C, shelf) | Not in v1. Added to TODO.md. |
| 4 | "Time left" | Sum of the remaining detected step timers. |
| 5 | End of recipe | A done screen with a star rating and an editable note. Finishing saves and returns to the detail view. |
| 6 | Persisted configuration | A new `config` table now. Settings pages come later (TODO.md). |

## State management: Jotai

Jotai (v3.0.1 on npm) holds UI state. TanStack Query holds server data (the database).

- **Packages:** `jotai` and `jotai-tanstack-query` (v0.11.0; peers `@tanstack/react-query` and `jotai >= 2`). `jotai-tanstack-query` lets an atom read a TanStack Query result, so the config value is loaded and cached by Query, and the atom exposes it.
- **Zod:** no package is needed. There is no dedicated Jotai–Zod integration on npm. Zod (already a dependency) validates values at the two boundaries: the config read from SQLite and the config written back. Jotai's storage docs describe the same pattern (custom storage validated with Zod), and form-atoms offers a Zod helper if forms ever move to atoms. Sources: https://jotai.org/docs/utilities/storage, https://github.com/form-atoms/form-atoms/issues/17, https://npmjs.com/package/form-atoms.
- **Verify before coding:** Jotai 3 is a major version. Check the `atomWithQuery` signature in `jotai-tanstack-query` 0.11 against Jotai 3 before writing atoms.

Ownership:

| State | Where it lives |
|---|---|
| Persisted configuration (text size, keep-awake, timer sound) | SQLite `config` table, read via TanStack Query (`atomWithQuery`), validated by Zod |
| Cook session (ingredient checks, done steps, running timers) | Jotai atoms, in-memory only, reset when cook mode opens |
| Current step | URL search param (`?step=`), as onboarding does |
| Recipe, tracks, schedule | TanStack Query, unchanged |

## Phases

### Phase 0: Dependencies

- Add `jotai`, `jotai-tanstack-query`, and `vitest` (5.0.5) with pnpm.
- Add a `test` script (`vitest run`) to `package.json`.
- Vitest runs in the Node environment with no DOM libraries. Only pure logic is tested in v1.
- Check that `tsc` in `pnpm build` still passes with the test files included, and that Vitest's version works with the installed Vite.
- Update `CLAUDE.md`: the commands section, the "no test framework" lines, and the state section (Jotai).

### Phase 1: Configuration table

Status: done. Migration `0006_config.sql`, `repo/config.rs` (`list`, `set`, with tests), commands `config_list` and `config_set`, `features/config/` (schema, query, mutation, atoms, schema tests), Jotai provider in `main.tsx`, and config loaded in the root route. Defaults: keep awake on, text size `md`, timer sound on.

- Migration `src-tauri/migrations/0006_config.sql`: `config (key TEXT PRIMARY KEY, value TEXT NOT NULL)`, with JSON-encoded values. A key-value table lets later settings pages add keys without migrations. Typed columns are the alternative if you prefer them.
- `src-tauri/src/repo/config.rs`: `get_all` and `set` (SQLx runtime queries). `commands.rs` gets `config_list` and `config_set`. Both are registered in `lib.rs`.
- Frontend: `src/features/config/` with
  - `schema.ts`: a Zod schema for the config, with defaults. A bad or missing value falls back to its default instead of breaking the page.
  - `queries.ts`: `configQueryOptions`, loaded in the root route loader, as other features do.
  - `mutations.ts`: `useSetConfig`, which invalidates the config query.
  - `atoms.ts`: `configAtom` (from `atomWithQuery`) and derived atoms per setting.

### Phase 2: Step logic with Vitest

Status: done. `steps.ts` (`durationsIn`, `timeLeft`) with `steps.test.ts` (9 tests). The Zod fallback tests live in `src/features/config/schema.test.ts`. The `test` script no longer uses `--passWithNoTests`.

- `src/features/recipes/cook/steps.ts`:
  - `durationsIn(text)`: returns every timed amount in the text, in minutes, as an array (empty when none). The array form lets one step hold several timers, e.g. "Cook 10 minutes, then rest 5 minutes". "20 minutes", "1 hour 15 min" and "12 minutes more" are each one entry. Ranges ("10–12 minutes") use the upper bound.
  - `timeLeft(steps, currentIndex)`: the sum of all detected durations from the current step to the end.
- `steps.test.ts` covers the parser, ranges, compound units, bad input, and `timeLeft`.

### Phase 3: Route and entry point

Status: done. Route `/recipes/$recipeId/cook` (`recipeCookRoute`, loads only the recipe) with `step` validated by `toCookStep`. `RootLayout` gives cook mode its own layout. `cook/atoms.ts` holds the session atoms, reset on each visit. `CookModeView` is a stub that Phase 4 replaces. The "Cook now" button is on the detail view, disabled with a tooltip when there are no instructions. The last step's Next is disabled until the done screen (Phase 7).

- `src/router.tsx`: add `/recipes/$recipeId/cook`. Its loader loads the recipe, as the detail route does. `RootLayout` exempts `/recipes/*/cook` from `DashboardLayout`, as it exempts `/onboarding`.
- `RecipeDetailView.tsx`: add a "Cook now" primary button. It is disabled, with a tooltip, when the recipe has no instructions.
- Opening cook mode resets the cook atoms.

### Phase 4: Desktop cook layout

Status: done. `CookModeView` composes `CookTopBar`, `IngredientChecklist`, `StepPanel`, `StepList` and `CookNav` in `components/cook/`, each with its CSS module. Ingredient checks and done steps use the atoms. "Next" marks the step done. Arrow keys and Escape work. The top bar's keep-awake switch and A-/A+ save to `config`; their effects come in Phases 6 and 8. The timer card is not here yet; it goes into `StepPanel` in Phase 5. Not compared visually with the design yet (needs `pnpm tauri dev`).

- `views/CookModeView.tsx` and CSS module, plus `components/cook/`: `CookTopBar`, `IngredientChecklist`, `StepPanel`, `StepList`, `CookNav`. Each has its CSS module next to it.
- Top bar: "Exit cook mode", the "COOKING NOW" label and recipe name, a segmented progress bar with "Step n of N", "About X min left", the keep-awake toggle, and A- / A+.
- Left: ingredient checklist with "n of N used".
- Centre: step heading, the timer card, and Previous / Next.
- Right: "All steps" list. Done steps are checked, the current step is highlighted, and upcoming steps are numbered. Clicking a step jumps to it.
- Keyboard: ← / → move between steps, Esc exits.

### Phase 5: Timers

Status: done. `cook/timers.ts` holds the pure timer logic (start, pause, +1 min, reset, due check, time format), with `timers.test.ts`. `timersAtom` holds timers by id. `useFinishTimers` checks once a second: a due timer gets a Mantine notification and a chime when `cook-timer-sound` is on. `TimerCard` sits in the centre column under the step, one card per timer in the step. A step with no duration gets a manual 5-minute timer. Timers reset on each visit to cook mode. Leaving cook mode stops the finish check, so a timer still running then will not notify. Chime playback depends on the webview allowing audio before a click.

- Jotai atoms for the timer list. Each running timer stores `endsAt` (a timestamp), so it stays accurate when the window is in the background.
- One timer per detected duration in the current step, plus a manual "Start" for steps without one. Start, pause, +1 min, and reset. Timers keep running when the user changes step.
- Finishing a timer shows an in-app notification and plays a short sound. OS notifications are out of scope.

### Phase 6: Keep awake

Status: done on Linux. Spike result: WebKitGTK 2.52.6 exposes `navigator.wakeLock`, but `request('screen')` rejects with `NotAllowedError` and never prompts. Rust-side keep-awake is used on all platforms, so there is one code path. Windows is not tested yet.

- `keepawake` crate 0.6.1 in `src-tauri/src/keep_awake.rs`. `KeepAwakeLock` holds the inhibitor in managed state. The `keep_awake_set` command turns it on or off. This is an app command, not a plugin, so no capability entry is needed.
- `cook/useKeepAwake.ts` turns it on while cook mode is open and the `cook-keep-awake` setting is on. Leaving cook mode releases it.
- The toggle is persisted in `config` (Phase 1).
- Linux uses `org.freedesktop.ScreenSaver.Inhibit`. The Rust test that takes the inhibitor passes here. Known risk: the crate's `Drop` calls `un_inhibit(...).unwrap()`, and the release profile uses `panic = "abort"`, so a failed release would abort the app.

### Phase 7: Done screen

Status: done. `?step=done` (validated by `toCookStep`) shows `CookDone` in the centre column. Next on the last step opens it, and the nav label there reads "Finish". The form uses `@tanstack/react-form` with zod, `StarInput` for the rating, and a notes field. "Save and finish" calls `useUpdateRecipe` only when the form is dirty, and then returns to the detail view. "Back to last step" returns to the final step. `recipeToInput` (`features/recipes/recipeInput.ts`, with tests) builds the save payload from the stored recipe, so no field is dropped. The edit view keeps its own form-values mapping, because it trims and parses strings. Escape leaves cook mode without saving, so typed notes are lost in that case.

- Shown when Next is pressed on the last step. It is a state of the cook route (`?step=done`).
- Star rating using the existing `StarInput`, and an editable notes field.
- "Finish" saves through `useUpdateRecipe`, only if the rating or notes changed, then returns to the detail view.
- The update needs the full recipe input. Extract a `recipeToInput(recipe)` helper from the edit view so both pages use the same mapping. This avoids losing fields on save.

### Phase 8: Text size

Status: done. `cook/textScale.ts` holds the sizes (`sm`, `md`, `lg`) and their fixed factors (0.875, 1, 1.25). `CookModeView` sets `--cook-text-scale` from the saved `cook-text-scale` setting on its root. The step heading, ingredient list, and all-steps list scale with it. A- / A+ save the new size to `config`, so it takes effect when the config query refetches. The factors are in a TypeScript module, not the Mantine theme, because they only apply to the cook page.

### Verification

- Each phase: `pnpm test` (Vitest) and `pnpm build`.
- Desktop: `pnpm tauri dev`, compared with `docs/designs/Cook now · desktop@1x.png`.
- `cargo test` for the config repository.

## Out of scope for v1

Mobile layout, oven chips, OS notifications, ingredient groups, settings pages. See TODO.md.
