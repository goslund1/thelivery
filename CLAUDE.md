# CLAUDE.md — Livery Catalog

Guidance for working in this repository. Read this before making changes.

## What this is

A catalog of Forza car liveries. It was **migrated from a single 17 MB HTML file**
(now in `archive/livery_catalog_edited.html`, kept locally / gitignored) into a
data-driven app:

- **`frontend/`** — Vue 3 + Vite + TypeScript + Pinia SPA.
- **`backend/`** — Rust (Axum) + SQLite (SQLx) API; in production it also serves the built SPA.
- **`tools/extract/`** — one-time Node script that parsed the original HTML into seed data + images.
- **`deploy/`** — systemd unit, deploy script, and Caddy/TLS setup for the DigitalOcean droplet.

Live at **https://thelivery.silverleaf.services** (Caddy terminates TLS, reverse-proxies to the backend).

## See also

- `.claude/skills/frontend-patterns/` — Vue subsystem internals (float panels, edit mode, slideshow, card list, theme builder, drawer system). Load when working on or extending these components.
- `.claude/skills/frontend-gotchas/` — Known Vue/CSS/TypeScript pitfalls. Load before debugging unexpected frontend behavior or writing new interactive components.
- `docs/plan.md` — Active work list and recently completed items.
- `docs/completed/DESIGN_SYSTEM.md` — Archived build-rules doc; its contents are folded into the skills above.

## The data model is the center of everything

One `Card` object (see `frontend/src/types.ts`) drives the DB rows, the API, the
Pinia store, and the components. The entity is **"card"** — a generic catalog
entry; the codebase is a generic card-gallery, not Forza-specific (only the UI
display copy still says "Livery"). A card's content is an **ordered, type-
dispatched `sections` array**, so new section types can be added without changing
the schema:

```ts
interface Card {
  id: string                 // "1".."6", "legend"
  catalogNumber: number
  name: string
  subtitle: string
  isFavorite: boolean
  isLegend: boolean          // the "legend" template card
  collections: string[]      // e.g. FH5, FH6, Drift, Street, Photo Safari
  tags: string[]
  images: CardImage[]        // lead/feature image === order 0 (no isLead flag)
  sections: Section[]
}

interface CardImage {
  id: number          // images table PK — stable across file moves; negative temp while pre-upload
  path: string        // resolved server-side from images table on every read
  thumbPath?: string
  stagePath?: string
  alt?: string
  order: number
  carId?: string | null
  liveryId?: number | null
}

type Section =
  | { type: 'text'; key; label; body; figurePath? }          // Inspiration, Design Notes
  | { type: 'forza_recipe'; key; label; tuneName; shareCode;
      coreSpecs: Record<string,string>;
      upgrades: { category; parts[] }[];
      adjustments: { name; description }[] }
```

Sections render through a dispatcher in `CardView.vue` (`v-if` on `section.type`
→ `TextSection.vue` / `RecipeSection.vue`). The section `key` (`inspiration`,
`notes`, `recipe`) is a stable slug used for the section filter and the dom id.
To add a section type: extend the `Section` union, the extractor, and the
`CardView` dispatcher. The backend stores the whole card as JSON, so it needs no
change. When adding a plain field, thread it through `types.ts` → the component;
it persists automatically.

## Running locally

Two processes. From the repo root:

```bash
# Terminal 1 — backend on :8787 (creates/seeds data.db on first run)
cd backend && cargo run

# Terminal 2 — frontend dev server on :5173 (proxies /api and /uploads to :8787)
cd frontend && npm install && npm run dev
```

Open http://localhost:5173. Vite's proxy (`frontend/vite.config.ts`) forwards `/api`
and `/uploads` to the backend, so the app always uses same-origin relative URLs.

**Reset to clean seed data:** stop the backend, `rm backend/data.db`, restart.

## Build / typecheck / verify

- **Frontend:** `npm --prefix frontend run build` runs `vue-tsc -b && vite build`. This is the typecheck gate — run it after frontend changes.
- **Backend:** `cd backend && cargo build`. For the production target: `cargo build --release --target x86_64-unknown-linux-musl` (needs `musl-tools`; set `CC_x86_64_unknown_linux_musl=musl-gcc`).
- **No automated test suite exists.** Verify by building, by curling the API, and by the production-simulation pattern (run the release binary with `FRONTEND_DIR`/`UPLOADS_DIR`/`SEED_PATH`/`DATABASE_PATH` set and curl `/`, `/api/cards`, `/uploads/...`).
- `frontend/shot.mjs` is a Playwright screenshot helper, but headless Chromium needs system libs (`libnspr4`, etc.) that require root to install — it won't run here without that.

## Backend (`backend/`)

- **Axum + SQLx (SQLite).** Single file: `backend/src/main.rs`.
- **Storage model:** one row per card in the `cards` table — `id`, `catalog_number`, and `body` (the full `Card` JSON). The `images` table is the **single source of truth** for image data; card body stores only `{ id, alt, order, carId }` per image — **no paths**. On every card read, `inject_images()` replaces body["images"] with the full rows from the DB (path, thumbPath, stagePath, livery_id, etc.). On every card write, `sync_card_images()` upserts the images table from the body and strips paths before saving. `normalize_bodies()` step 3 migrates legacy cards at startup (idempotent).
- **Endpoints:** `GET/PUT/POST/DELETE /api/cards[/:id]`, `POST /api/images` (multipart upload, accepts `livery_id` field → returns `{ id, path, ... }`), `GET /api/cars` (search), `GET/POST /api/liveries`, `POST /api/admin/liveries/:id/assess-color` (auth-gated, calls Claude with thumbnail, stores primary/secondary color), `GET /api/health`, static `/uploads/*`, and (production) the SPA at everything else.
- **Serving the SPA:** `ServeDir::new(FRONTEND_DIR).not_found_service(ServeFile::new(index.html))`. Real files (index, hashed assets) serve at 200; unknown paths return index.html with a 404 status — acceptable because the app has **no client-side router** (only `/` is a real entry point). Don't "fix" this with `ServeDir::fallback` — that broke static serving entirely in this tower-http version.
- **Config via env** (set by the systemd unit in prod):
  - `BIND_ADDR` (default `0.0.0.0`; prod `127.0.0.1`), `PORT` (default `8787`)
  - `DATABASE_PATH`, `UPLOADS_DIR`, `SEED_PATH`, `FRONTEND_DIR`
- **Seeding:** on first run, if `cards` is empty, it imports `backend/seed/cards.json`. A startup `normalize_bodies()` migrates any old-shape rows (the `liveries`→`cards` rename + inspiration/designNotes/recipe → `sections[]`, dropping `isLead`) — idempotent.
- **Migrations:** SQLx migrations in `backend/migrations/`. **Never edit an applied migration — always add a new one** (`sqlx migrate add <desc>`). This matches the global rule about migrations.

## Frontend (`frontend/src/`)

### Stack & layout
- Vue 3 `<script setup>` SFCs, Pinia stores, TypeScript. Entry: `main.ts` (registers Pinia + the global `v-tip` directive).
- **State lives in Pinia, never in the DOM** (the original was the opposite). Two stores:
  - `stores/cards.ts` (`useCardsStore`) — the `Card[]` data + mutations + API calls (`load`, `save`, per-card snapshots).
  - `stores/ui.ts` — theme, text size, edit mode, expand/collapse, filters, which modal is open, and the **per-card dirty set**.

### CSS — important
- **All styling is one global stylesheet, `src/styles/catalog.css`, copied verbatim from the original HTML.** Components reuse those exact class names. **Do not rename classes or convert to scoped styles** — visual parity depends on the global rules. Scoped `<style>` is only for genuinely new bits (e.g. the per-card save button, unsaved-count).
- **Themes:** `data-theme` on `<html>` swaps ~35 CSS variables; 5 themes (dark/light/rainbow/clouds/stormy). Two live knobs: `--text-delta` (text scaling) and `--dissolve` (crossfade).

### Component tree
`App.vue` → `SideBug` (+ `Filters` slot), `EditBar`, and a `v-for` of `CardShell`/`CardView` (all filtered cards always mounted — see `.claude/skills/frontend-patterns/` → Card list rendering), plus global modals (`Lightbox`, `ChipPicker`, `ImagePicker`, `ExitConfirmModal`, `CustomTip`).
`CardView` → `CardMeta`, `Gallery`, `TagCloud`, then a `CollapsibleSection` per `card.sections` entry, dispatched by type to `TextSection` / `RecipeSection`. Reusable: `EditableText`.

## Images

- Files live under `backend/uploads/`, served at `/uploads/*`.
- **`images` table is the single source of truth** — `id INTEGER PRIMARY KEY AUTOINCREMENT`, `card_id`, `path`, `thumb_path`, `stage_path`, `car_id`, `alt_text`, `sort_order`, `livery_id`. Card body JSON stores only `{ id, alt, order, carId }` — no paths. Paths are resolved server-side by `inject_images()` on every card read.
- `POST /api/images` (multipart) accepts `file`, `card_id`, and optionally `livery_id`; resizes to thumb + stage variants; inserts an `images` row immediately; returns `{ id, path, thumbPath, stagePath }`. `id` is the integer PK — always use it as the stable image identifier, never the path.
- `CardImage.id` on the frontend is a `number` (DB PK). The only valid negative value is a temp id (`--_imageIdCounter`) used in `addImageToPool` for the brief window before the upload response arrives. All component logic (`Gallery.vue`, `ImagePicker.vue`, `PhotoDetail.vue`, `cards.ts`) treats image id as `number`.
- The seed images were decoded from the original HTML's base64. **The data URIs claimed `image/png` but the bytes are JPEG** — `tools/extract/extract.mjs` sniffs magic bytes for the real extension. Re-run extraction only if needed (`cd tools/extract && npm run extract`); it reads `archive/livery_catalog_edited.html` (kept locally, not in the repo).
- `backend/uploads/` is **tracked in git** (the seed set), so a fresh clone runs as-is. Production deploys **never overwrite** `data.db` or uploaded images (seed images are copied in only if missing).

## Deployment

- **Push to `main`** triggers `.github/workflows/deploy.yml`: builds the static musl backend + the frontend, bundles them with seed data + the systemd unit, rsyncs to the droplet, and runs `deploy/remote-deploy.sh` (idempotent installer).
- In production the **single binary** serves API + `/uploads` + SPA on `127.0.0.1:8787`; **Caddy** (installed once via `deploy/setup-caddy.sh <domain>`) terminates TLS on 443 and reverse-proxies to it, auto-renewing the Let's Encrypt cert.
- Required GitHub secrets: `DEPLOY_SSH_KEY`, `DEPLOY_HOST`, `DEPLOY_USER` (passwordless sudo). The droplet runs the service as a dedicated `thelivery` user under `/opt/thelivery`.
- After a push, you can confirm the live build updated by checking the asset hash in `frontend/dist/index.html` against `https://thelivery.silverleaf.services/`.

## Conventions & rules

- **Git:** never auto-commit; commit only when asked (this repo's branch is `main`). End commit messages with the `Co-Authored-By` trailer.
- **Staging before committing:** always run `git status` before any commit to see the full picture of modified and untracked files. The codebase often has inter-dependent files in flight (types, api, components, backend) — committing only the files you touched and leaving the rest behind will break CI. Stage everything that's part of the same feature together.
- **Migrations:** new migration files only; never edit applied ones.
- **Don't break visual parity:** keep `catalog.css` and its class names intact; the original single-file app in `archive/` is the reference to diff against.
- After frontend changes, run `npm --prefix frontend run build` (typecheck) before considering it done; after backend changes, `cargo build`.
- Keep edits same-origin and relative (`/api`, `/uploads`) — never hardcode hosts/ports in the frontend.
