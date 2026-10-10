# Theme Panel — Spec

Status: starting doc, not yet built. Written against the current `CLAUDE.md`
(Vue 3 + Pinia frontend, Rust/Axum + SQLite backend). Supersedes any earlier
theme-panel discussion that assumed a different stack.

This satisfies the pending feature already listed in `CLAUDE.md`:
> Theme builder — planned as a slide-in panel launched from the Themes flyout
> in SideBug; waiting for mockup from Jason.

## Two modules, two scopes

| | Site theme | Card accent |
|---|---|---|
| Scope | Global, one config | Per card, optional |
| Entry point | `SideBug` → existing Themes flyout, extended | New edit-only affordance in `CardMeta` (mirrored in `EditCardModal` / `NewCardModal`) |
| Persisted via | `localStorage`, same as the existing 5 themes and text-size knob | Part of the `Card` JSON body, via the existing `PUT /api/cards/:id` |
| New backend work | None | None — it's a new optional key on the existing JSON blob |
| Default | One of the 5 existing presets | `inherit` (no override) — most cards carry zero extra data |

No new API endpoints either way. That's deliberate — both ride existing
persistence paths instead of adding new ones.

## Module A — Site theme

**Where it lives:** this is the existing Themes flyout in `SideBug`
(dark/light/rainbow/clouds/stormy), extended with a 6th option, **Custom**,
that opens the slide-in builder panel instead of just swapping `data-theme`.

**What it edits:** a curated subset of the ~35 theme CSS variables — not all
of them in v1. Proposed starting set, based on what actually showed up when
the live site's `:root` was inspected:

- `--asphalt` (page background)
- `--panel`, `--panel-edge`, `--panel-well` (card/panel surfaces)
- `--gold`, `--gold-bright`, `--magenta` (accents)
- `--paper` (primary text), `--steel`, `--steel-light` (muted text)
- `--danger`, `--success` (status colors — probably leave these alone in v1,
  list them so the panel doesn't silently miss anything if someone goes
  looking for them)

**Not in v1, flag as open questions:**
- **Fonts.** `catalog.css` doesn't currently expose font-family as a theme
  variable — body/label fonts are hardcoded. Adding font choice means adding
  new CSS variables to `catalog.css` first; this is new infrastructure, not
  just exposing something that already exists. Recommend descoping fonts
  from v1 unless you want that work now.
- **`--text-delta` and `--dissolve`.** These already exist as separate live
  knobs (text-size slider, crossfade speed). Could fold them into the same
  panel since they're already theme-adjacent — your call, no new plumbing
  either way.

**Persistence:** store the custom palette as a 6th theme value in
`localStorage`, same mechanism as the existing presets — this matches the
precedent already set by upgrade presets ("per-browser, not per-card"). Don't
add a backend table for this unless you specifically want a logged-in user's
custom theme to follow them across devices — there is a user system already,
so that's possible later, just not required for v1.

**Example shape** (not a new endpoint — just what gets written to
`localStorage` under e.g. `theme:custom`):

```json
{
  "asphalt": "#0b0b0d",
  "panel": "#15151a",
  "panelEdge": "#23232b",
  "gold": "#c9a227",
  "magenta": "#d6478f",
  "paper": "#ece9e4",
  "steel": "#7a7e87"
}
```

## Module B — Card accent override

**Where it lives:** follows the existing "edit-only affordance" convention —
rendered always in markup, hidden via `body.editing-mode` (CSS), not
conditionally mounted. Lives in `CardMeta` next to the chip add/remove and
lead-star controls, and mirrored in `EditCardModal` / `NewCardModal` for
section parity, same as everything else in those modals.

**What it edits:** a single optional field on `Card`:

```ts
accentOverride: { mode: 'inherit' | 'gold' | 'magenta' | 'custom'; color?: string } | null
```

- `null` / `inherit` → card visually uses whichever accent the active site
  theme defines. This should be the default for nearly every card.
- `'magenta'` is the Smokin' lane. Worth considering: since `Drift` is
  already a real `collections` value, the override picker could *suggest*
  magenta when `Drift` is present rather than requiring a manual pick every
  time — still overridable, just a sane default instead of a blank choice.

**Persistence:** no new endpoint. It's a new key on the same JSON body that
already round-trips through `PUT /api/cards/:id`.

## Open decision: badge indicators (Abide / Victory / Smokin')

The earlier mockup included checkboxes for these as card metadata. Worth
reconsidering before building: `CLAUDE.md` is explicit that this codebase is
deliberately generic, not Forza-specific — `tags[]` and `collections[]`
already exist as free-form string arrays, and `ChipPicker` already does
add/remove UI for them.

Two paths:

1. **Lower churn:** treat `abide` / `victory` / `smokin` as known tag values
   in the existing `tags[]` array. No schema change, no new component — reuse
   `ChipPicker`/`TagCloud` as-is. Loses dedicated icon rendering unless you
   special-case those three tag strings in `TagCloud` for display.
2. **More bespoke:** a dedicated field/section type with its own icons and
   placement rules. More control, more surface area to maintain, and cuts
   against the "generic card gallery" framing in `CLAUDE.md`.

Recommend (1) unless you specifically want the icon treatment — flagging it
rather than deciding it for you, since it's a real design call.

## Open decision: default-expanded section

`ui.ts` already has a global expand/collapse knob. A per-card,
per-section "open by default" setting (from the earlier mockup) is new scope
beyond that, not an extension of it. Worth confirming whether the global
toggle already covers the need before adding a second, more granular
mechanism that does something similar.

## Checklist before Claude Code builds this

- [ ] Confirm the v1 variable list above (add/cut anything)
- [ ] Decide font theming: in scope now, or descoped
- [ ] Decide badge indicators: tags (low churn) vs dedicated field (bespoke)
- [ ] Decide default-expanded: skip, or build alongside the existing
      expand/collapse knob
- [ ] Confirm `localStorage`-only persistence for custom theme is fine for v1
