# Theme Builder — Plan

Two separate modules, two separate trigger points, two separate config payloads.
Do not combine these into one panel or one JSON blob — they have different scope,
different persistence, and different audiences.

---

## Module A — Site theme

**Scope:** global. One config, applies to the whole app.  
**Trigger:** the existing moon/theme icon on the SideBug rail. This panel replaces
the current Dark/Light/Rainbow/Clouds/Stormy dropdown — ambiance becomes one
select inside the panel, not the only control.  
**Auth:** edit-mode only. Visitors read the current theme; only the logged-in
admin can change it.  
**Persisted:** single-row `theme` table in SQLite. Endpoints: `GET /api/theme`
(public, called on app load) and `PUT /api/theme` (auth-required).

### A1 — Main palette controls

| Control | CSS variable | Notes |
|---|---|---|
| Page background | `--asphalt` | |
| Card / panel | `--panel` | |
| Panel border | `--panel-edge` | |
| Gold accent | `--gold` | derived tokens auto-follow via `color-mix()` |
| Magenta accent | `--magenta` | derived tokens auto-follow via `color-mix()` |
| Primary text | `--paper` | |
| Muted text | `--steel` | |
| Label / mono face | font select | JetBrains Mono, IBM Plex Mono, Space Mono, Roboto Mono |
| Display face | font select | placeholder — verify current live font first |
| Stage ambiance | select | Dark / Light / Rainbow / Clouds / Stormy (existing behavior) |

### A2 — CSS derived token refactor (prerequisite)

Before the builder can ship, harden the derived tokens so changing a base color
cascades automatically. Convert all hardcoded tint strings in `catalog.css` to
`color-mix()`:

```css
/* before */
--gold-bright:   #e0b430;
--gold-chip:     #cc9a1f;
--gold-tint-04:  rgba(201,162,39,0.04);
--gold-tint-06:  rgba(201,162,39,0.06);
--gold-tint-14:  rgba(201,162,39,0.14);
--magenta-tint-06: rgba(214,71,143,0.06);
--magenta-tint-40: rgba(214,71,143,.4);

/* after */
--gold-bright:     color-mix(in srgb, var(--gold) 85%, white);
--gold-chip:       color-mix(in srgb, var(--gold) 88%, black);
--gold-tint-04:    color-mix(in srgb, var(--gold)  4%, transparent);
--gold-tint-06:    color-mix(in srgb, var(--gold)  6%, transparent);
--gold-tint-14:    color-mix(in srgb, var(--gold) 14%, transparent);
--magenta-tint-06: color-mix(in srgb, var(--magenta)  6%, transparent);
--magenta-tint-40: color-mix(in srgb, var(--magenta) 40%, transparent);
```

Browser support: Chrome 111+, Firefox 113+, Safari 16.2+. No polyfill needed.
After this change, `setProperty('--gold', '#e83d9c')` cascades to all tints
automatically. The store only needs to set the 7 base color variables.

### A3 — Tuning palette

Nine per-category accent colors, independently swappable from the main palette.
Lives as a collapsible section inside Module A (not a separate panel).

Current hardcoded values → snapped to FH palette as part of this work:

| Category | Current | FH palette color |
|---|---|---|
| Tires | `#00d4f5` | Speed flare cyan `#29C5F6` |
| Gearing | `#0fc4a0` | Danger sign teal `#1FD1A5` |
| Alignment | `#c870d0` | Festival night pink `#E63DD0` |
| ARB | `#8c5cf6` | Drift zone purple `#8A2BE2` |
| Springs | `#f5906a` | Horizon orange `#F4831F` |
| Damping | `#e85d2a` | Sunset orange `#E8650F` |
| Aero | `#84cc16` | Battle green `#5BDB2E` |
| Brakes | `#f04060` | Bright Tokyo red `#FF3B2F` |
| Differential | `#6884f8` | Speed trap blue `#1E6FE0` |

The tuning palette snap is a free cleanup — do it as part of Phase 0 alongside
the `color-mix()` refactor, before any UI work starts.

### A — JSON shape

```json
{
  "colors": {
    "asphalt": "#0b0b0d",
    "panel": "#15151a",
    "panelEdge": "#23232b",
    "gold": "#c9a227",
    "magenta": "#d6478f",
    "paper": "#ece9e4",
    "steel": "#7a7e87"
  },
  "tuning": {
    "tires": "#29C5F6",
    "gearing": "#1FD1A5",
    "alignment": "#E63DD0",
    "arb": "#8A2BE2",
    "springs": "#F4831F",
    "damping": "#E8650F",
    "aero": "#5BDB2E",
    "brakes": "#FF3B2F",
    "differential": "#1E6FE0"
  },
  "fonts": {
    "mono": "JetBrains Mono",
    "display": "Archivo Black"
  },
  "ambiance": "dark"
}
```

---

## Module B — Per-card theme override

**Scope:** per card. Optional and nullable — absence means "inherit site theme."
Do not default to an explicit copy of the current site values or every card
silently locks in today's colors.  
**Trigger:** a palette icon on the card's edit-mode rail (close / edit / save /
palette). Only visible in edit mode.  
**Persisted:** a nullable sub-object on the card record. No DB migration needed —
the backend stores cards as whole JSON; absent fields default gracefully.

### B — Controls

| Control | Type | Notes |
|---|---|---|
| Accent preset | radio | Inherit site theme (default) / Gold / Magenta / Custom |
| Custom accent | color picker | conditional — only shown when preset = Custom |
| Badge: Abide | checkbox | display-only metadata flag |
| Badge: Victory | checkbox | flags Jason's custom tune |
| Badge: Smokin' | checkbox | flags drift-series identity |
| Default expanded section | select | None / Inspiration / Design notes / Tune-build parts |

### B — JSON shape

No override (default):
```json
{
  "themeOverride": null,
  "display": {
    "badges": { "abide": false, "victory": false, "smokin": false },
    "defaultExpanded": "none"
  }
}
```

With override:
```json
{
  "themeOverride": { "accent": "#FF3B2F" },
  "display": {
    "badges": { "abide": true, "victory": false, "smokin": true },
    "defaultExpanded": "inspiration"
  }
}
```

`accent` is either `"gold"`, `"magenta"`, or a literal hex string for Custom.

### Inheritance rule

Card rendering = site theme as base, with `themeOverride.accent` swapped in
for `--gold` only if present. Panel colors, fonts, ambiance, and tuning palette
always come from the site theme — no per-card override for those.

---

## The color picker component

One reusable component (`ColorPicker.vue`) used everywhere a color is set:
Module A palette fields, Module A tuning fields, Module B custom accent.

### Layout

```
┌─────────────────────────────────┬──────────────────┐
│  2D saturation/brightness       │  FH palette       │
│  square (hue on X, sat/bri      │  swatches         │
│  on Y axes)                     │  (scrollable)     │
│                                 │                   │
│  ── Hue slider ──────────────   │  ● Reds           │
│  ── Alpha slider (optional) ─   │  ● Oranges        │
│                                 │  ● Yellows        │
│  Hex  [ #C9A227        ]        │  ● Greens         │
│  R [ 201 ] G [ 162 ] B [ 39 ]  │  ● Teals/Blues    │
│                                 │  ● Purples/Pinks  │
│                                 │  ● Neutrals       │
└─────────────────────────────────┴──────────────────┘
```

### Three-way sync

- **Swatch click** → loads color into 2D picker, fills hex + RGB fields
- **Drag 2D picker / hue slider** → hex + RGB update live
- **Type hex** → picker jumps to position, nearest swatch highlights if exact match
- **Type RGB** → same as hex

Saved value is always the final hex — not a reference to the swatch name.
Nudging a swatch-loaded color produces a new hex; no swatch stays highlighted.

### Swatch active state

A small dot indicator on a swatch lights up only when the current hex exactly
matches that palette color. If you've nudged from a starting swatch, no dot is
active — you're in custom territory, which is honest.

A "snap to nearest" affordance (small icon next to the hex field) returns to
the closest FH palette color if you've drifted and want back.

### Contrast warning

A small ⚠ badge appears on the swatch or next to the hex field when the chosen
color's contrast ratio against the current `--panel` background falls below
WCAG AA (4.5:1). Informational only — does not block the choice.

### Implementation

Use `vanilla-colorful` (~2.5kb, framework-agnostic) for the 2D picker widget.
Wrap it in `ColorPicker.vue` and add the swatch rail. One component, used
everywhere.

---

## FH color palette (27 colors)

Organized by hue family for the swatch grid:

**Reds**
| Name | Hex |
|---|---|
| Bright Tokyo red | `#FF3B2F` |
| Rising sun red | `#D6432C` |
| Lantern red | `#C81E3A` |
| Deep maroon | `#8C2A22` |

**Oranges**
| Name | Hex |
|---|---|
| Sunset orange | `#E8650F` |
| Horizon orange | `#F4831F` |

**Yellows / Golds**
| Name | Hex |
|---|---|
| Gold | `#C9A227` |
| Horizon Tour gold | `#EAA63C` |
| Checkpoint yellow | `#F5D033` |

**Greens**
| Name | Hex |
|---|---|
| Eliminator neon green | `#7FFF3C` |
| Battle green | `#5BDB2E` |
| Circuit green | `#3FBE3E` |
| Forest eliminator green | `#2A9D4A` |
| Deep eliminator green | `#167A3E` |

**Teals / Cyans**
| Name | Hex |
|---|---|
| Danger sign teal | `#1FD1A5` |
| Speed flare cyan | `#29C5F6` |

**Blues**
| Name | Hex |
|---|---|
| Speed trap blue | `#1E6FE0` |
| Indigo night sky | `#2A2F6B` |

**Purples / Pinks / Magentas**
| Name | Hex |
|---|---|
| Drift zone purple | `#8A2BE2` |
| Festival night pink | `#E63DD0` |
| Hot pink | `#D6478F` |
| Style flare magenta | `#FF2D7A` |
| Sakura pink | `#F2A6C8` |

**Neutrals**
| Name | Hex |
|---|---|
| Pure white | `#FFFFFF` |
| Panel slate | `#2B2B33` |
| Menu charcoal | `#16161A` |
| Checkered black | `#0A0A0A` |

---

## Store

New `useThemeStore` (`stores/theme.ts`). Does not live in `useUiStore` — it has
its own persistence lifecycle (loads from API, saves to API, applies CSS vars).

Responsibilities:
- Load `GET /api/theme` on app start (alongside `store.load()` in `App.vue`)
- Apply all base color variables via `document.documentElement.style.setProperty()`
- `color-mix()` in CSS handles derived tokens automatically — the store only sets
  the 7 base colors + 9 tuning colors
- `save()` action PUTs the full JSON to `/api/theme`
- `setColor(key, value)` applies instantly (live preview), marks dirty
- `reset()` reverts to last saved state

---

## Future — Guest theme adjuster

Not built in this phase. Noted here so the architecture doesn't foreclose it.

Visitors will eventually be able to nudge colors and have their choices remembered.
Persistence: **a cookie** (not localStorage) so the server can read it on every
request and the admin can sample guest choices. Guest-set themes are never
automatically promoted to the named theme — promotion is an explicit admin action.

The `ColorPicker` component is scoped so Module A (admin-only) and a future guest
adjuster can use the same component with different persistence targets.

---

## Implementation phases

### Phase 0 — CSS hardening (no UI, prerequisite)
- Convert tint variables to `color-mix()` in `catalog.css`
- Snap `--tabc-*` variables to FH palette values
- Verify nothing breaks visually

### Phase 1 — Backend
- New `theme` table (single row, JSON body)
- `GET /api/theme` (public)
- `PUT /api/theme` (auth required)
- Seed with current dark theme defaults

### Phase 2 — Store + color picker component
- `stores/theme.ts` (`useThemeStore`)
- `ColorPicker.vue` (vanilla-colorful + FH swatch rail + hex field)
- Wire `GET /api/theme` into app startup

### Phase 3 — Module A panel
- Replace SideBug theme dropdown with slide-in panel
- Main palette section (7 color pickers + font selects + ambiance)
- Tuning palette section (9 color pickers, collapsible)
- Save / revert actions

### Phase 4 — Module B (per-card)
- Add `themeOverride` + `display` fields to `Card` type and backend
- Palette icon on card edit rail
- Accent preset + custom picker + badges + defaultExpanded controls

---

## Open items

- Display font: verify the actual current font-family used for card titles on the
  live site before wiring the font selector
- `DESIGN_SYSTEM.md`: create alongside this work — document the full token set,
  which are editable vs. derived, and the FH palette with intended usage notes
- Module B badges (Abide / Victory / Smokin') are new card fields — confirm final
  names and meanings before building the UI
- Mobile rail: both module triggers (moon icon, palette icon) are on the SideBug
  rail which has known mobile clipping issues — solve mobile separately
