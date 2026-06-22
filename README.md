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

## Deployment

Pushing to `main` triggers `.github/workflows/deploy.yml`, which:

1. builds a static `x86_64-unknown-linux-musl` release binary (no glibc-version
   issues on the droplet),
2. builds the Vue frontend (`npm run build`),
3. bundles binary + `static/` (the SPA) + `seed/liveries.json` + seed images +
   the systemd unit, rsyncs it to the droplet, and runs `deploy/remote-deploy.sh`.

In production the **single Rust binary serves everything** — the API, `/uploads`,
and the SPA — on **port 80**, managed by systemd as the `thelivery` service.

### One-time droplet prerequisites

- `rsync` installed (`apt-get install -y rsync`).
- The SSH/deploy user has **passwordless sudo** (the deploy script creates the
  service user, installs the unit, and restarts the service).
- Port **80** open in the droplet firewall / DO cloud firewall.
- (The `thelivery` service user, `/opt/thelivery`, and the systemd unit are
  created automatically on the first deploy.)

### Required GitHub repository secrets

| Secret | Value |
|---|---|
| `DEPLOY_SSH_KEY` | Private SSH key authorized on the droplet (the full PEM) |
| `DEPLOY_HOST` | Droplet IP or hostname |
| `DEPLOY_USER` | SSH user with passwordless sudo |

### After deploy

```bash
ssh <user>@<host> 'systemctl status thelivery'   # service health
ssh <user>@<host> 'journalctl -u thelivery -n 50'  # logs
```

Persistent state on the droplet lives at `/opt/thelivery/data.db` and
`/opt/thelivery/uploads/`; deploys never overwrite them (seed images are copied
in only if missing). **HTTPS is not included** — front it with Caddy, nginx, or
Cloudflare if you want TLS.

## Database migrations

SQLx migrations live in `backend/migrations/`. To change the schema, add a new
migration (never edit an applied one):

```bash
cd backend
sqlx migrate add <description>
```
