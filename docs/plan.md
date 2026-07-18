# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.

---

## Active — ordered by priority

### 1. Jason's first OG Maker road test
First real first-party use of the OG Maker. Open a card, click the title → ShareModal → OG Designer. Try building a text box from scratch: pick a font, set a style (POSTCARD / SIGNAL / GHOST), type content, move/resize, save as preset, and "Save to Card." Goal: surface any UX friction in the add-box flow and validate the 12-font library with actual usage before doing any more flow work.

---

### 2. Mobile layout
Narrow-screen pass for the full catalog. Known gaps:
- Theme builder flyout — doesn't fit on small screens
- General card layout on narrow viewports
- No blocking dependencies remaining

---

## Maintenance

### Live data gaps (from session-36 shakedown)
- ~~Color filters match zero cards on live~~ **RESOLVED** — the live data was fine (11/11 card-referenced liveries had colors); the real bug was the liveries store never loading outside edit flows, so the filter's liveryId→color lookup was empty for fresh visitors. Fixed by `loadAll()` on Filters mount (commit a70cce3, verified live).
- ~~Set JWT_SECRET on the droplet~~ **RESOLVED** — was confirmed missing (every deploy rotated the fallback secret and logged all users out; bit us twice on 2026-07-17). Geoff added it to `/opt/thelivery/secrets.env` same day; verified by a token surviving a subsequent deploy restart.
- ~~Orphan livery rows~~ **CLEANED 2026-07-17** — ids 26–28 "Smokin (Updated)" deleted (no images, no live or purgatory card references). Kept: 24 "Faker" (belongs to purgatory card id 11 — needed for restore) and 29 "Sold Out" (assessed colors, provenance unclear).
- **Test the admin delete/purge flow on the purgatory cards** — "Rally Base Car Test" (id 22) and "Faker" (id 11) are being kept in purgatory specifically as test subjects for a later admin-delete/restore/purge shakedown.
- **Seed-data nits** — Smokin engine spec reads `DIRECT_TEST`; Lolita tune name is still the placeholder with share code TBD.
- ~~Create Geoff's editor account on live~~ **DONE** — Jason created it via Add User; as of 2026-07-18 Geoff hasn't signed in yet (temp password stays valid until he completes the forced change).

### User management gaps (from session-38 audit follow-up)
- **No way to list users** — no `GET /api/users` route and no UI, so an admin can't see what accounts exist (came up when verifying Geoff's account; only workaround is trying to re-create the username and reading the "already taken" error). Add an admin-only list route + a read-only list in Account Settings showing username, role, and the `must_change_password` flag (doubles as a "hasn't signed in yet" indicator).
- **`user.create` isn't audit-logged** — `create_user` in `backend/src/auth.rs` never calls `audit::record`, so the Timeline has no record of account creation. Trivial fix: one `audit::record(..., "user.create", "user", Some(username), Some(json!({"role": role})))` call after the insert.

### Pre-launch checklist
- **Lock CORS to production domain** — currently `CorsLayer::permissive()` in `backend/src/main.rs`. Change to `CorsLayer::new().allow_origin("https://thelivery.silverleaf.services")` before public launch.
- **Remove the "Draft — layout preview" tag** from the page head in `App.vue` — do together with the CORS lock.
- **Update README.md** — significantly out of date: still references `/api/liveries` (now `/api/cards`), `seed/liveries.json` (now `seed/cards.json`), "single-user, no auth" (JWT auth exists), and is missing most current endpoints. Rewrite the API table and data description to match reality before the repo goes public.
- ~~Set JWT_SECRET on the droplet~~ **DONE 2026-07-17** — Geoff set it in `secrets.env`; verified by a token surviving a deploy restart (see Live data gaps above).

### Backfill pass (another round coming)
- Cars→Tunes hierarchy refactor is complete (migration 0017, normalize_bodies step). Data model is now stable.
- Ready for a backfill pass via EditCardModal when content work resumes.

### AI billing notification (deferred — wait until credits actually run out)
- When `assess-color` hits a 429 or quota error, the toast already surfaces it in-app. That's enough for now.
- If a proactive alert becomes needed: `NOTIFY_WEBHOOK` env var in the systemd unit; backend POSTs on quota error; check if Anthropic API exposes a balance endpoint to poll. No frontend changes needed.

### Deferred
- **One-click revert on the Timeline tab** — audit entries already store everything needed (`detail.prev` JSON for livery/tune/theme/og-preset overwrites, `detail.prevVersion` linking card_history for card edits); a revert endpoint + button would make reversal self-serve instead of manual.
- **Editor suggestion triage** — suggestions moderation is admin-only because dismissing is a hard delete. If Geoff should triage, dismiss needs to become a soft delete first.
- **`car_colors`** — factory *stock* color options per car model (e.g. "this Corvette comes in Arctic White, Rapid Blue..."). Requires scraping Forza wikis. Not to be confused with `primary_color`/`secondary_color` on the `liveries` table, which is the AI livery color assessment already wired into the import flow. No ETA on car_colors.

---

## Recently completed

- **Code audit pass 5** — scope: everything since pass 4, correctness + security focus on the roles/temp-password/audit-timeline batch. Six fixes: `create_card` 409s on id collisions instead of upsert-overwriting (protects purgatoried cards from silent resurrect-and-overwrite; frontend retries with the next free catalog number); `put_card` now 404s unknown ids and 409s purgatoried ids (same resurrection hazard — a stale editor tab saving after an admin soft-delete would un-delete the card); `GET /api/cards/:id` filters `deleted_at` (was the only card read leaking purgatoried content publicly); `create_user` validates username (trimmed, 1–64 chars); login burns equivalent Argon2 work on unknown usernames (timing-based username enumeration); `login_rate_log` pruned on failed logins (rows >1 day old). Plus a comment on the assess-color route (deliberately editor-accessible despite the `/api/admin` prefix) and handler-level tests for both card guards. Clean otherwise: admin gating consistent, temp-password gate server-enforced, audit logging covers all editor mutations. — 2026-07-17

- ~~**User roles + audit timeline**~~ **DONE** (totally planned all along) — migration 0020: `users.role` admin/editor; `AdminUser` extractor 403s editors on all `/api/admin/*`, hard deletes (livery/tune/preset/og-preset), trash purge, and `POST /api/users` (closed a hole where any authed user could mint accounts). Editors keep card CRUD incl. soft delete, image upload/trash-delete, livery/tune/car create+update, presets, theme, assess-color. Every editor-capable mutation logs to `audit_log` with prev-state JSON for overwrites and card_history version links for card edits; Admin Panel gained a read-only Timeline tab. Role read from DB per request → demotion is immediate. — 2026-07-17 (commit f79ef0c, AAR: `docs/aar-2026-07-17d.md`)

- ~~**Temp-password onboarding**~~ **DONE** (also "planned") — migration 0021: `must_change_password` flag; while set, backend 403s everything except `PUT /api/me/password`. Add User form gained role select + "temporary password" checkbox (default on); non-dismissable `ForcePasswordModal` (mounted last in App.vue, deliberately outside the modal store so Escape/closeTopModal can't reach it) forces the change on first sign-in; sign-out keeps the temp password valid for retry. Smoke-tested end-to-end. — 2026-07-17 (commit 7a42481, AAR: `docs/aar-2026-07-17d.md`)

- **SPA cache headers — stale-bundle fix** — Live shakedown "found" serious bugs (tune-tab collapse, section state loss, phantom gaps, dead Light theme) that all turned out to be a stale browser-cached bundle from before the virtual-scroller removal; the deployed code was already correct. Root cause: `index.html` served with no `Cache-Control`, so browsers heuristic-cache it and keep old hashed bundles after deploys. Fixed with `spa_cache_control` middleware on the SPA service only: `no-cache` for index.html/fallback (revalidates via Last-Modified → cheap 304s), `public, max-age=31536000, immutable` for hashed `/assets/*`; `/api` and `/uploads` untouched. Deployed and verified live. Genuinely still open from the shakedown: **color filters match zero cards on live** (liveries have no color data — run the assess-color pass against live), and seed-data nits (Smokin engine `DIRECT_TEST`, Lolita placeholder tune name / TBD share code). — 2026-07-17 (commit 74798e9)

- **Security batch: login rate limit + traversal fix + og:url** — `/api/login` rate-limited (5 failed/5min, 20/hour per anonymized /24; DB-backed `login_rate_log`, migration 0019); `is_safe_upload_path()` rejects `..`/`.` components in `trash_image` (the old `starts_with` check passed `uploads/../x` — lexical comparison); `client_ip()` honors X-Forwarded-For so per-IP buckets survive the Caddy proxy (applied to login + suggestions — socket address alone would have made the limit global behind the proxy); `og:url` now absolute. — 2026-07-17 (AAR: `docs/aar-2026-07-17c.md`)

- **Chrome shakedown — live site + wipe-and-reseed** — Full pass done. Found and fixed: Light theme half-applying (flyout now routes through `setAmbiance()`); live DB missing cars/users/liveries seed data → deploy pipeline now ships the full seed set (`USERS_SEED_JSON` secret for users) and live was wiped (backup: `data.db.bak-20260717` on droplet) and re-seeded from local, including purgatoried cards via the seed-only `deletedAt` marker; three fresh-DB seeding bugs (numeric image ids, livery FK, normalize resurrecting soft-deleted cards); gallery + zero-result-filter empty states. — 2026-07-17 (AAR: `docs/aar-2026-07-17c.md`)

- **Quality pass: module split + backend test suite** — full codebase read-through, then per-commit cleanup: CardView stale `variants[]`→`cars[]` fix; main.rs (3363 lines) split into domain modules (`auth`/`cards`/`images`/`trash`/`identity`/`share`/`suggestions`/`presets`/`theme`/`state`); light card-body validation on PUT/POST (schema-free JSON preserved — unknown fields/sections pass through); `list_cards` N+1 → single grouped query (output verified byte-identical); theme.ts static ui import (never was a cycle); ui↔modal circular imports marked SETTLED (comment + gotchas entry); `.env` gitignored; **29-test suite** covering the startup migration pipeline (`normalize_card`, `migrate_variants_to_cars`, `ensure_standard_sections`, `sync_card_images` branches, end-to-end `normalize_bodies`) against in-memory SQLite. Backend gate is now `cargo test`. 21 finished docs moved to `docs/completed/`. — 2026-07-17 (AAR: `docs/aar-2026-07-17b.md`)

- **Code audit pass 4** — six fixes across compositor, OG Maker, ShareModal, auth: `@input` on content field for live preview; scrim overlap check; `as any` removed from ShareModal; `list_og_presets` auth-gated; `render_glyphs` + `blit_with_transform` extracted (−75 lines); `TextStyle` enum + `OgTextStyle` union replace magic strings. Deferred: circular store import warning (`ui.ts` ↔ `modal.ts`). — 2026-07-16 (commit b3214b2, AAR: `docs/aar-2026-07-16b.md`)

- **OG overlay access fixed** — ShareModal OG design section now gated by `auth.isAuthenticated` instead of `ui.isEditing`. Admins see the overlay composer in normal user view (click card title → Share modal) — the correct flow since sharing is a post-edit action, and the title is an editable input in edit mode. Also added `/share` to the Vite dev proxy so the compositor preview and share routes work in local dev. — 2026-07-16

- **OG Overlay Studio** — server-generated 1200×630 `og:image` for social share unfurls (Discord, iMessage, Reddit). Pure-Rust compositor (`fontdue` + `image` crate): `POST /share/preview` for live OG Maker preview, `GET /share/:id/card.png` for public card image, `og_presets` table + CRUD. Three text styles: POSTCARD (Bebas Neue, floating), SIGNAL (Oswald VF + dark chyron backdrop), GHOST (Oswald VF, semi-transparent). Polish: bottom scrim gradient, per-style drop shadows, logo slot ("THE LIVERY" placeholder). OG Maker modal: freely-positioned text boxes with corner-resize, rotation handle, shear slider; 200ms debounced live preview; preset save + "Save to Card." ShareModal integration with preset picker, preview thumbnails, Adjust/Reset flow. — 2026-07-16

- **CarTabs/TuneTabs shakedown + polish**: discipline baseline presets (Race/Rally/Drift/Street) seeded in DB; `+ ADD CAR` / `+ ADD TUNE` buttons styled as dashed tabs (grey→gold/pink on hover); TuneTabs shelf color = `--highlight`; spacing and gap fixes; `v-tip-up` directive for right-flying tooltip from text end; card title opens ShareModal. — 2026-07-13
- **Social sharing foundation + Reddit pre-fill**: OG share page endpoint (`GET /share/:id/:slug`) added to Rust backend — server-rendered HTML with full OG + Twitter card tags; real browsers get `<meta refresh>` to `/`. Card title opens `ShareModal` (view mode only, `v-tip-up` tooltip). Modal: Copy Link button (clipboard API, 2s feedback) + **Post to Reddit** (pre-fill approach — opens `reddit.com/submit?url=…&title=…` in new tab with editable title pre-populated from card + car name + share code; Reddit's native form handles subreddit selection and posting) + Discord stub. Note: direct Reddit API posting blocked — Reddit ended self-service OAuth access Nov 2025 for personal projects. See `docs/plan-reddit-share.md`. — 2026-07-13

- **CarTabs shakedown + bug fixes**: spurious Springs and Dampers dialog on tab switch fixed (`_inPropUpdate` flag + `flush:'sync'` on gearCount watcher in TuningAdjustments — prevents prop-driven gearCount change from calling checkImplied); tune-name placeholder added to EditableText (CSS `::before` + `data-placeholder` attr) so empty tuneName in edit mode shows the car model name as hint text. — 2026-07-12
- **Gear preset fix + preset kind** (migration 0016): `applyPresetValues`, `executeApplyPreset`, and `saveAsPreset` all now include locked gear rows — previously skipped them, so presets saved/applied without Race Transmission had no gear values. `getAdjustments()` now includes locked rows so gear values survive the flush→rebuild cycle. `applyImpliedTransmission()` added: infers required transmission from highest `gearN` key in preset, auto-adds it via `implied-upgrades` emit (mirrors `onTransChoice` pattern). Rally preset (id=6) was saved while locked and has no gear keys — needs a re-save after adding Race Transmission. Migration 0016 adds `kind TEXT NOT NULL DEFAULT 'build'` to `tuning_presets`; Baseline checkbox in save dialog; `◆` prefix on baseline entries in dropdown. Baseline/build conflict logic (skip baseline-locked rows when applying a build preset) deferred until first real baseline preset exists. — 2026-07-11
- **RefImg pipeline** (migration 0015): `image_role` + `included` columns on images table; `included` now persists across page reloads (was ephemeral). RefImg images get structured filenames `{card_slug}_RefImg{NN}_{date}_{uuid6}`, default to excluded from slideshow, skip livery creation + color assess. CarPicker gets `+IMG` button (`showImageBtn` prop, dashed style) that emits `select-image` — no car search. NewCardModal tracks `imageRole`, shows RefImg chip, passes role to uploads. `sync_card_images` now syncs `included` back to DB on card save. — 2026-07-11
- **NewCardModal car/recipe parity**: `RecipeSection` gets `forceEdit` prop so all edit-mode fields (CarPicker, spec table, upgrades, preset bar) appear in NewCardModal without requiring global edit mode. — 2026-07-11
- Replaced `vue-virtual-scroller` with plain `v-for` (all filtered cards always mounted). Virtual scroll's slot-pool model is incompatible with stateful Vue components — two failure modes (slot recycling + concurrent duplicate slots) required module-level singleton workarounds that added permanent complexity. At catalog scale the memory cost of mounting all cards is negligible. `useCardVisibility.ts` (IntersectionObserver + KeepAlive) left in place for when the catalog is large enough to need lazy mounting. See `CLAUDE.md` → "Card list rendering" — 2026-07-10
- Settings/Admin UI reorganization: Account panel (change password + Add User gated behind toggles, Admin Panel → button, Sign Out); separate Admin panel with Tools tab (image tools, stats, orphans, trash, seed) + Export Card tab (YAML download/import, legacy repair at bottom); Tune Suggestions stays in Filters flyout only — 2026-07-07
- figurePath patching in migration flow: ImageMigrationModal snapshots old paths and patches TextSection.figurePath after migrate; Repair Figure Paths endpoint matches by sequence number before falling back to lead image — 2026-07-07
- Trash compactor: orphans scan moves to trash instead of hard-delete; user-deleted images move to trash on card save; GET/DELETE /api/admin/trash + POST /api/admin/trash/restore; trash_log table (migration 0012); reason badges, select/restore/delete — 2026-07-06
- Security & quality audit: fixed orphan scanner (queried card body instead of images table), delete_images legacy variant naming, orphan scan skips uploads/trash/, rate limit HashMap growth, suggestion adjustments 64KB cap, e:any catches — 2026-07-06
- Car Tabs wizard: floating modal walks through each non-anchor car, user picks a tuning preset per car, creates tabs with pendingPresetId, auto-applies on first tab open — 2026-07-08
- Tab strip UX polish: delete 2→1 leaves last tab visible, delete last collapses to clean state + re-activates Car tabs button, short model-name labels with full name on hover title — 2026-07-08
- Folder tab visual styling: gold shelf via container border-bottom, all tabs margin-bottom:-1px, active tab border-bottom-color:var(--panel) to erase shelf beneath it — 2026-07-08 (NOTE: took ~1hr and significant token burn due to Claude inventing z-index/pseudo-element schemes instead of researching the standard CSS pattern first. Jason's feedback: research known UI patterns before writing code, present the approach for sign-off before executing.)
- CollapsibleSection HMR sync fix: immediate:true on store watch so expand/collapse state survives hot reload — 2026-07-08
- Multi-car mashup foundation: tab strip, auto-propose banner, + button consolidation, gallery carId filtering — 2026-07-06
- Image migration pipeline: full re-file + rename, all 11 cards migrated, structured filenames, drum CarPicker, toast drawer, dotenvy, FH6 FD cars, Bronco R — 2026-07-06
- Car identity model: cars table, CarPicker, PhotoDetail, livery picker, AI color assess, Step 8 hardening — 2026-07-06
- Image identity refactor: images table as source of truth, integer PKs, inject_images on read, sync on write, ImageMigrationModal — 2026-07-05
- Suggest bar two-tier trigger, card accent override, per-section defaultOpen, RecipeSection one-way flow, code quality passes 1 + 2 — 2026-07-05
- Transmission/gearing system, TuningAdjustments Define Stock + undo, ThemeBuilder effects sliders — 2026-07-04
- Theme builder + ColorPicker, DrawerPanel, card history, YAML export/import, suggest bar two-surface redesign — earlier sessions
