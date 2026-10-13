# thelivery — backlog

Open items only, one line each. A finished item leaves this file: its
story goes to the day's file in `spec/history/`. Split out of
`docs/plan.md` on 2026-10-09 (frozen: `spec/history/verbatim/2026-10-09-plan.md`).

Next free ID: B-18

## Next

- **B-1** **Jason's first OG Maker road test** — First real first-party use of the OG Maker. Open a card, click the title → ShareModal → OG Designer. Try building a text box from scratch: pick a font, set a style (POSTCARD / SIGNAL / GHOST), type content, move/resize, save as preset, and "Save to Card." Goal: surface any UX friction in the add-box flow and validate the 12-font library with actual usage before doing any more flow work.

## Found during the bottom bar work order, 2026-10-09

- **B-2** **The app is blank when the browser blocks storage** (e.g. Safari with all cookies blocked). Confirmed in the harness, before and after Part B: `stores/auth.ts` and `api.ts` read `localStorage` at start with no guard, and the throw stops the app mounting. The visitor memory is guarded and is not the cause. Other unguarded uses: `useFactoidSchema.ts`, `ColorPicker.vue`, `ShareModal.vue`, `OgMakerModal.vue`, the write in `useAssessFailures.ts` and `RecipeSection.vue`.
- **B-3** **Hover paints the "on" look** on two admin controls (B5 sweep): `.sv-btn--like` in `SuggestionViewer.vue` and `.tb-ambiance-btn` in `ThemeBuilder.vue` show the same accent on hover as when active, so a click that switches one off shows no change while the cursor stays. Same cure as the star (suppress until mouseleave, or a distinct hover).
- **B-4** **`.fav-star.pop` is not wired**: `catalog.css` has the star's pop animation from the original; nothing in `src/` sets the class.

## Live data gaps (from session-36 shakedown)

- **B-5** **Test the admin delete/purge flow on the purgatory cards** — "Rally Base Car Test" (id 22) and "Faker" (id 11) are being kept in purgatory specifically as test subjects for a later admin-delete/restore/purge shakedown.
- **B-6** **Seed-data nits** — Smokin engine spec reads `DIRECT_TEST`; Lolita tune name is still the placeholder with share code TBD.

## Pre-launch checklist

- **B-7** **Lock CORS to production domain** — currently `CorsLayer::permissive()` in `backend/src/main.rs`. Change to `CorsLayer::new().allow_origin("https://thelivery.silverleaf.services")` before public launch.
- **B-8** **Remove the "Draft — layout preview" tag** from the page head in `App.vue` — do together with the CORS lock.
- **B-9** **Update README.md** — significantly out of date: still references `/api/liveries` (now `/api/cards`), `seed/liveries.json` (now `seed/cards.json`), "single-user, no auth" (JWT auth exists), and is missing most current endpoints. Rewrite the API table and data description to match reality before the repo goes public.

## Content and maintenance

- **B-10** **Backfill pass (another round coming)** — Cars→Tunes hierarchy refactor is complete (migration 0017, normalize_bodies step). Data model is now stable. Ready for a backfill pass via EditCardModal when content work resumes.
- **B-11** **AI billing notification (deferred — wait until credits actually run out)** — When `assess-color` hits a 429 or quota error, the toast already surfaces it in-app. That's enough for now. If a proactive alert becomes needed: `NOTIFY_WEBHOOK` env var in the systemd unit; backend POSTs on quota error; check if Anthropic API exposes a balance endpoint to poll. No frontend changes needed.

## From the Shakedown hosting order, 2026-10-12

- **B-17** **`deploy/setup-caddy.sh` would wipe Shakedown's path if rerun.** It writes the whole Caddyfile (The Livery's site only), so running it again on the droplet would overwrite Geoff's hand-set `/shakedown/` setup (Shakedown's page, served from `/srv/shakedown`, live since 2026-10-12). Caddy on the droplet is Geoff's, set by hand; nothing of ours writes it. Updating the script to match is Geoff's (Jason, 2026-10-12). Until then, don't rerun it.

## Deferred

- **B-12** **Rate limiting on `GET /api/cards`** — auth and suggestions already have DB-backed rate limiting; the open card read endpoint has none. A scraper can pull the full catalogue as fast as they like. Low urgency at current card count, but worth adding before the catalogue scales. Same pattern as `suggestions.rs`.
- **B-13** **Prerendering / SSR for SEO** — the SPA's `index.html` is a JS shell; crawlers that don't execute JS see nothing. The `/share/` pages are already server-rendered (best current SEO asset). A lightweight SSR fallback �� Axum detects bot User-Agents and returns a pre-rendered HTML snapshot of the card list — would make the main page indexable without a framework rewrite. Needs a scoping conversation with Geoff before any work starts.
- **B-14** **One-click revert on the Timeline tab** — audit entries already store everything needed (`detail.prev` JSON for livery/tune/theme/og-preset overwrites, `detail.prevVersion` linking card_history for card edits); a revert endpoint + button would make reversal self-serve instead of manual.
- **B-15** **Editor suggestion triage** — suggestions moderation is admin-only because dismissing is a hard delete. If Geoff should triage, dismiss needs to become a soft delete first.
- **B-16** **`car_colors`** — factory *stock* color options per car model (e.g. "this Corvette comes in Arctic White, Rapid Blue..."). Requires scraping Forza wikis. Not to be confused with `primary_color`/`secondary_color` on the `liveries` table, which is the AI livery color assessment already wired into the import flow. No ETA on car_colors.
