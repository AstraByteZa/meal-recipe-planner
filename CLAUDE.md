# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

Package manager is pnpm (`pnpm-lock.yaml`).

- `pnpm dev` — Vite dev server on port 1420 (strict; fails if taken). Tauri expects this port.
- `pnpm build` — runs `tsc` (type check, `noEmit`) then `vite build`. This is the only type-check and lint gate; there is no ESLint script.
- `pnpm test` — runs the Vitest suite (`vitest run`). Test files are `*.test.ts` next to the code they cover. v1 tests pure logic in the Node environment, with no DOM libraries.
- `pnpm preview` — serve the built frontend.
- `pnpm tauri dev` / `pnpm tauri build` — run or bundle the desktop app. Tauri runs `pnpm dev` / `pnpm build` itself via `src-tauri/tauri.conf.json`.

To run a single frontend test file, use `pnpm test <path>` (for example `pnpm test src/features/recipes/cook/steps.test.ts`).

## Architecture

This is a Tauri v2 desktop app: a React + TypeScript frontend in `src/` rendered in a webview, and a thin Rust backend in `src-tauri/`.

**Frontend bootstrap.** `src/main.tsx` mounts the app with a Mantine theme (custom `myColor` palette, dark color scheme by default) and the TanStack Router. Mantine CSS imports are order-sensitive: the `@mantine/schedule` styles must come after core and dates.

**Routing.** `src/router.tsx` builds the route tree by hand. The TanStack router Vite plugin is configured with `enableRouteGeneration: false` and `autoCodeSplitting: true`, so new routes must be added to `routeTree` manually. The root route renders `DashboardLayout` (a sidebar `NavbarSearch` plus `<Outlet />`) for every page except `/onboarding`, which has its own layout. Its `beforeLoad` redirects to `/onboarding` until `settings.onboarded` is set, so the redirect is skipped for `/onboarding` itself. The index route `/` shows the meal planner.

**Features.** Feature code lives under `src/features/<feature>/` with `views/` for route-level components and `components/` for pieces they use. The only feature is `planner`: `MealPlannerView` (week header and an "Auto fill gaps" button, which has no handler yet) renders `PlannerSchedule` (a Mantine `SimpleGrid` of seven day columns and a Breakfast row). The schedule grid and past weeks read tracks and meals from the database through the `/` route loader (`src/features/planner/queries.ts`). `FillTheGaps` and the navbar `WeekProgress` still use placeholder data in `data/mockPlan.ts`. Track settings (name, default time, show/hide, repeat days, color, order) are edited in the `manageTracks` modal and saved with `tracks_save`, which replaces the whole list in one transaction; tracks dropped from the list lose their planned meals. Hidden tracks are left out of the grid, and a track's repeat days decide which grid cells are open.

**Dates.** Use `src/lib/dayjs.ts` rather than importing `dayjs` directly. It registers the `isoWeek` plugin, which the planner depends on for week boundaries.

**Backend.** `src-tauri/src/lib.rs` builds the app, opens the database in `setup`, and registers `tauri-plugin-opener` and the commands. Commands live in `commands.rs` as thin wrappers over `repo/` (SQL, one module per feature). `db.rs` opens SQLite at `app_data_dir/planner.db` with a `SqlitePool` (sqlx, no ORM) and runs `src-tauri/migrations/*.sql`. Add a migration as a new numbered file; never edit an applied one. Errors go through `AppError` in `error.rs`. A recipe saved by name only has empty `url` and `source_domain` (not NULL). Do not rebuild the `recipes` table to make them nullable: with foreign keys on, dropping it cascades into `schedule_entries` and `recipe_tags`. Tests are in-module `#[cfg(test)]` blocks using `db::test_pool()` (in-memory); run with `cargo test` in `src-tauri/`. The template `greet` command is only used by `src/App.tsx`, which is dead code not imported by `main.tsx`. Capabilities (permissions the webview may use) are declared in `src-tauri/capabilities/default.json`; a new plugin or core API needs a permission there as well as in `Cargo.toml` and `lib.rs`.

**Styling.** Mantine components with `*.module.css` files alongside components. `postcss.config.cjs` applies `postcss-preset-mantine` and defines the Mantine breakpoint variables.

## Best practices

- **Styling priority:** use CSS modules (`*.module.css`, imported as `classes`) for component styling. Use Mantine style props only when a component needs one or two style values, such as `mt="sm"` or `p="sm"`. If a component needs more than that, move the styles into its module.
- Keep CSS modules next to the component that uses them, named after it (`UserButton.tsx` and `UserButton.module.css`).
- Use Mantine's theme (`myColor`, breakpoints) for colors and sizes rather than hardcoding hex values or pixel breakpoints in CSS.
- Target design: `docs/designs/schedule.png` (weekly meal schedule) and `docs/designs/recipe_links.png` (recipe library). Match these layouts when building the planner and recipe UI.

### Routing and data fetching

- **No `Suspense` in route components.** TanStack Router handles async itself: data is loaded in the route `loader`, and `pendingComponent` shows while it runs. Do not wrap route components in `<Suspense>`.
- **Load data in the route loader with `context.queryClient.query(options)`.** Do not use `ensureQueryData`; it is deprecated in `@tanstack/query-core` 5.105. Pass `staleTime: 'static'` only if the cache should never refetch on navigation.
- **Read data in the view with `useSuspenseQuery(options)`.** The loader has already filled the cache, so the view does not suspend.
- **Keep loader inputs in search params, not `useState`.** When a view's data depends on UI state (such as the shown week), validate it with `validateSearch`, pass it to the loader with `loaderDeps`, and change it with `navigate({ search })`. The planner's `week` param is the example.
- **Put query options in `src/features/<feature>/queries.ts`** using `queryOptions({ queryKey, queryFn })`. For Tauri data, `queryFn` calls `invoke` from `@tauri-apps/api/core`.
- The `QueryClient` lives in `src/lib/queryClient.ts`. Router context carries it (`createRootRouteWithContext<{ queryClient }>`), and `main.tsx` provides it through `QueryClientProvider`.
- Routes set `errorComponent` and `pendingComponent` as needed. Shared loading UI is `src/components/RouteLoader`.

### Modals and forms

- **Modals use the Mantine modals manager** (`@mantine/modals`). Each modal is a context modal component, registered by name in `src/features/planner/modals/registry.ts` (the only `MantineModalsOverride` declaration, passed to `ModalsProvider` in `main.tsx`). Open one with `modals.openContextModal` from a small `open<Name>Modal(...)` helper next to the component (`openAddMealModal` in `src/features/planner/modals/AddMealModal.tsx`). Do not render modal dialogs inline in views.
- **Forms use `@tanstack/react-form` with `zod`.** Define the schema with `z.object` next to the form, pass it as `validators: { onChange: schema }`, and render fields with `form.Field`. Map Select and other Mantine inputs with `field.state.value`, `field.handleChange`, and `field.handleBlur`. Show errors only after the field is touched.
- **Client state uses Jotai; server data uses TanStack Query.** Jotai atoms hold UI state that lives only in the browser, such as a running cook session. Data from the database stays in TanStack Query. An atom can read a query through `jotai-tanstack-query`'s `atomWithQuery`. Validate values that cross the Tauri boundary with a `zod` schema.
- **Writes go through a mutation hook** in `src/features/<feature>/mutations.ts` (`useMutation` with `invoke`). On success, invalidate the matching query prefix so the views refetch (`["schedule"]` for the planner). Drag and drop uses `@dnd-kit` (see `ManageTracksModal.tsx`). Use `mutate` with `onSuccess` in a form, not `mutateAsync`, so a failed save shows an error instead of an unhandled rejection.
