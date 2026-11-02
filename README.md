# 🍽️ Mise

**A personal meal planner for our family.** Plan the week's meals, keep the recipes we actually cook, and get a shopping-friendly view of what's on the menu. Built as a desktop app, with optional sync between our devices.

> 🚧 **Work in progress.** This is a personal project and an experiment. It is under active development, features are incomplete, and things will change or break. It is shared in case it's useful to others, but it is not a finished product. Use it at your own risk, and keep your own copy of any recipes you care about.

---

## Why this exists

Every week we ended up asking the same question: *"What are we eating?"* Recipes were scattered across bookmarks, screenshots and memory, and the plan lived in somebody's head. Generic meal planning apps either wanted a subscription, pushed their own recipe catalogue, or didn't fit how our household actually works: different meals for kids and adults, different days of the week, and dinners that repeat.

So this app was built for us. It is shaped around one household's routine, and it's built to be opened every day, not just once a week.

## What it does

- **Weekly schedule.** A grid of seven days with a row per meal track (Breakfast, Lunch, Dinner and so on). Tracks can be renamed, reordered, recoloured, hidden, or set to repeat only on certain days.
- **Recipe library.** Save recipes by name, or paste a link to a recipe you found online. Tag them, add details, and keep them in one place.
- **Drag-and-drop planning.** Move meals around the week with drag and drop, or add them from your recipe library.
- **Cook mode.** Step-by-step cooking view with timers and an in-app alert when a timer finishes.
- **Past weeks.** Look back at what we ate.
- **Guided onboarding.** A short first-run setup for the household, the meal tracks, and a first recipe and plan.
- **Works offline.** Everything is stored locally in SQLite. No account is needed.
- **Optional sync and family sharing.** Sign in to sync between devices through a self-hosted sync server, and invite family members to share a plan. Private data stays private to its owner.

### Planned, not built yet

- **Auto-fill gaps:** suggest meals for empty slots, based on household size, dietary needs and what we've cooked recently.
- **Recipe scraping:** pull the ingredients and steps straight from a recipe website.
- **Recipe images** on recipe links.
- **Week start setting** (Monday or Sunday) applied across the planner.

Open items and decisions are tracked in [`docs/TODO.md`](docs/TODO.md).

## How it's built

| Layer | Tech |
| --- | --- |
| Desktop shell | [Tauri v2](https://tauri.app/) (Rust) |
| UI | React, TypeScript, [Mantine](https://mantine.dev/) |
| Routing and data | TanStack Router, TanStack Query, TanStack Form |
| Local database | SQLite via [sqlx](https://github.com/launchbadge/sqlx) |
| Sync server | Rust, [Axum](https://github.com/tokio-rs/axum), PostgreSQL |
| Shared code | `shared-core` crate for sync types and conflict rules |

The repository is a Cargo workspace with three members: the desktop app (`src-tauri`), the sync server (`sync-server`), and the shared sync code (`shared-core`). Each device keeps its own database and syncs changes through the server using last-write-wins conflict resolution.

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org/) 24 and [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- The [Tauri system prerequisites](https://tauri.app/start/prerequisites/) for your OS

### Run the app

```sh
pnpm install
pnpm tauri dev
```

### Other useful commands

```sh
pnpm test            # frontend unit tests (Vitest)
pnpm build           # type check and production frontend build
cargo test           # Rust tests (sync server tests need TEST_DATABASE_URL)
pnpm tauri build     # bundle the desktop app
```

### Run the sync server locally

```sh
cargo run -p sync-server   # needs DATABASE_URL and JWT_SECRET (32+ bytes)
```

Or with Docker, including Postgres:

```sh
docker compose -f sync-server/docker-compose.yml up --build
```

Deployment notes are in [`docs/deploy-dokploy.md`](docs/deploy-dokploy.md).

## Releases

Pushing a version tag such as `v0.1.0` builds the Linux and Windows installers through GitHub Actions and creates a **draft** release with them attached. Review the draft and publish it from the Releases page.

Installers are unsigned for now, so Windows SmartScreen will show a warning on first run.

## Project layout

```
src/                 React frontend (features, routes, UI)
src-tauri/           Tauri app: Rust commands, SQLite repo layer, migrations
shared-core/         Sync wire types, LWW rules, deterministic IDs
sync-server/         Axum sync server with Postgres
docs/                Design mockups, plans, deploy guide, TODO
```

## Design

The target look for the schedule, recipe library, onboarding, and family screens is in [`docs/designs/`](docs/designs/).

## License

No license has been chosen yet. All rights reserved by the author until one is added.
