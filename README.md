# Livery Catalog

A catalog of Forza car liveries — migrated from a single 17 MB HTML file into a
Vue 3 app with a Rust persistence API.

```
archive/   the original single-file app (livery_catalog_edited.html), retired
tools/     one-time extraction script (HTML -> seed JSON + image files)
backend/   Rust + Axum + SQLite API (single-user, no auth)
frontend/  Vue 3 + Vite + TypeScript + Pinia
```

## Run it

**Backend** (http://localhost:8787):

```bash
cd backend
cargo run            # creates data.db, runs migrations, seeds from seed/liveries.json on first run
```

**Frontend** (http://localhost:5173):

```bash
cd frontend
npm install
npm run dev          # proxies /api and /uploads to the backend
```

Open http://localhost:5173.

## Data

- The catalog lives in SQLite at `backend/data.db` (one row per livery; the full
  object is stored as JSON). Images are files under `backend/uploads/`, served at
  `/uploads/*`.
- Edits made in the app's edit mode are saved via `PUT /api/liveries/:id` and
  persist across reloads and restarts.
- **Reset to clean seed data:** stop the backend, delete `backend/data.db`,
  restart.

## Re-extracting from the original (rarely needed)

The seed data and images were generated once from `archive/livery_catalog_edited.html`
(that original file is kept locally and is not committed to the repo):

```bash
cd tools/extract
npm install
npm run extract      # regenerates backend/seed/liveries.json and backend/uploads/
```

## API

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/liveries` | list (ordered by catalogNumber) |
| GET | `/api/liveries/:id` | one livery |
| PUT | `/api/liveries/:id` | save (whole-object upsert) |
| POST | `/api/liveries` | create (body must include `id`) |
| DELETE | `/api/liveries/:id` | delete |
| POST | `/api/images` | multipart upload, returns `{ path }` |
| GET | `/uploads/*` | static images |

## Database migrations

SQLx migrations live in `backend/migrations/`. To change the schema, add a new
migration (never edit an applied one):

```bash
cd backend
sqlx migrate add <description>
```
