# Design System & Build Rules — Livery Catalog

A distilled rulebook sitting next to `CLAUDE.md`. `CLAUDE.md` describes what
the codebase *is*; this describes the patterns to follow when adding
*anything new*, so new features land in the existing shape instead of
needing a revision pass. When in doubt, this doc should make "where does this
go" and "how is this shaped" answerable without guessing.

## 1. The data model is generic — don't make it Forza-specific

The whole point of the `Card` + typed `sections[]` model is that it's a
generic card gallery, not a Forza tool that happens to render in a browser.
Concretely:

- New **structural** content types → extend the `Section` union, the
  extractor, and the `CardView` dispatcher.
- New **plain fields** → thread through `types.ts` → the component. The
  backend stores the whole card as JSON, so no migration is needed for this.
- Resist adding Forza-specific or brand-specific fields (e.g. a dedicated
  `abideBadge: boolean`) when the existing generic arrays (`tags[]`,
  `collections[]`) already cover the need. Ask "could this be a tag?" before
  adding a new typed field.

## 2. State lives in Pinia, never in the DOM

This was a deliberate fix from the original single-file app, where state and
DOM were tangled. Two stores, two jobs:

- `stores/cards.ts` — the data: `Card[]`, mutations, API calls, per-card
  snapshots (baselines for discard-on-exit).
- `stores/ui.ts` — everything else: theme, text size, edit mode,
  expand/collapse, filters, which modal is open, the per-card dirty set.

New global, app-wide state → `ui.ts`. New persisted data → `cards.ts` /
the `Card` shape. Don't introduce a third store unless a feature genuinely
doesn't fit either bucket.

## 3. Per-card vs global is a real architectural fork, not a style choice

This came up directly while speccing the theme panel, and it generalizes:

| Question | If global | If per-card |
|---|---|---|
| Where does the control live? | A flyout/panel off `SideBug` | An edit-only affordance inside `CardMeta` / the edit modals |
| How does it persist? | `localStorage` (matches theme, text-size, upgrade presets) unless multi-device sync is explicitly needed | A new optional key in the `Card` JSON body, via the existing `PUT /api/cards/:id` |
| Default value | One of the existing presets | `null` / `inherit` — most cards should carry zero extra data |

Before building any new control, answer "is this one setting for the whole
app, or one setting per card?" first — it decides almost everything else
about where it lives and how it's saved.

## 4. CSS — one stylesheet, never scoped, theming via variables only

- All styling lives in `src/styles/catalog.css`, copied verbatim from the
  original app. **Never rename existing classes or convert them to scoped
  styles** — visual parity is checked against `archive/livery_catalog_edited.html`.
- Scoped `<style>` is only for genuinely new UI with no prior art (e.g. the
  per-card save button).
- Theming is `data-theme` on `<html>` swapping CSS variables — never inline
  style overrides for theme-able properties. If a new control needs to be
  theme-aware, it needs a CSS variable, not a hardcoded color.
- **Edit-only affordances are always rendered, hidden via `body.editing-mode`**
  (chip add/remove, lead-star, contenteditable styling all work this way).
  New edit controls should follow this pattern — render in markup always,
  gate visibility with the existing class — rather than `v-if`-mounting them
  only in edit mode. Keeps the pattern consistent and avoids a second way of
  doing the same thing.

## 5. Known structural gotchas (don't rediscover these)

- **`scrollIntoView` is banned** for the thumbnail rail — `block:'nearest'`
  walks the scroll chain and jumps the whole page. Set `scrollLeft` directly.
- **`vue-tsc` doesn't count string template refs as used.** If a composable
  needs an element ref, create it in the component and pass it in (see
  `Gallery.vue` → `useSlideshow`).
- **Don't use `ServeDir::fallback`** in the backend — it broke static serving
  in this tower-http version. The current `not_found_service` approach is
  intentional, not a workaround waiting to be cleaned up.
- **Never edit an applied SQLx migration** — always `sqlx migrate add`.
- **`normalize_bodies()` is idempotent on purpose** — it's the seam that
  migrates old-shape rows. New shape changes to the card body should extend
  this function rather than relying on a one-time manual fix.
- **`overflow-x: auto` implies `overflow-y: auto`** — CSS spec: the two
  overflow axes cannot have one `auto` and the other `visible`. Setting
  `overflow-x: auto` on any element unexpectedly makes it a vertical scroll
  container, causing trackpad bounce. Always pair with `overflow-y: hidden`
  when horizontal-only scroll is intended.
- **`pointer-events: none` also kills `:hover`** — using it to make a control
  non-interactive also silently removes all hover feedback. Use `opacity` +
  `cursor: default` when the element should still look interactive but
  do nothing, or accept that hover is gone when you use `pointer-events: none`.
- **Virtual scroll: components aren't destroyed when off-screen** —
  `DynamicScroller` uses `visibility: hidden` and `transform: translateY()`,
  not mount/unmount. Slots ARE recycled and can be assigned to a different
  item or duplicated simultaneously. Any display-mode state (open/closed, active
  tab, stacked/unstacked) must live in a module-level singleton (`Ref` keyed by
  cardId), not in `<script setup>`. See `stackedState.ts` for the pattern.

## 6. Persistence precedent — check before adding a new mechanism

There are already three different persistence patterns in use. Pick the one
that matches, don't invent a fourth without reason:

1. **Per-card, server-side** — part of the `Card` JSON body, `PUT /api/cards/:id`.
   Use for anything that belongs to a specific card and should survive across
   browsers/devices.
2. **Global, client-side** — `localStorage`. Used for theme, text-size,
   upgrade presets. Use for app-wide preferences that don't need to follow a
   user across devices.
3. **Global, server-side** — doesn't exist yet for settings (only cards and
   users are server-side). Only reach for this if a global setting genuinely
   needs to sync across devices/users — it's new infrastructure, not a free
   choice.

## 7. Branding marks (Abide / Victory / Smokin') are content rules, not schema

These are rules about what gets painted into the livery and where, not
fields the app needs to track structurally:

- **Abide** — universal artist signature, adapted per build, usually rear.
- **Victory** — crossed-flags tuning-quality mark, front fenders, only on
  cars carrying Jason's custom tune (the game can't enforce this pairing).
- **Smokin'** — drift-series identity (cursive, clouds, magenta), reserved
  for drift-spec builds.

If the catalog needs to *display* whether a card carries these, prefer
representing them as tag values in the existing `tags[]` array (see the
theme-panel spec's "badge indicators" section for the tradeoff) rather than
adding dedicated fields, unless there's a clear reason to special-case them.

## 8. Before building anything new — checklist

- [ ] Is this global or per-card? (Section 3) — decides where it lives and
      how it's saved.
- [ ] Does it need a new `Section` type, or is it a plain field on the
      existing `Card` shape? (Section 1)
- [ ] Does it belong in `ui.ts` or `cards.ts`? (Section 2)
- [ ] Could it reuse an existing generic mechanism (`tags[]`, `collections[]`,
      `localStorage` presets) instead of a new bespoke one? (Sections 1, 6)
- [ ] If it's theme-able, is it wired through a CSS variable, not a hardcoded
      value? (Section 4)
- [ ] If it's an edit-only control, is it rendered always and hidden via
      `body.editing-mode`, matching every other edit affordance? (Section 4)
- [ ] Does it touch any of the known gotchas in Section 5?
- [ ] Does the persistence choice match one of the three existing patterns,
      or is a new one genuinely justified? (Section 6)
