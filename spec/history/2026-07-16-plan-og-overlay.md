# OG Overlay Studio — Plan

Social share links unfurl into a preview card on Discord, Reddit, iMessage, and everywhere else that reads Open Graph tags. The `og:image` is what they show. Right now that's either nothing or a raw photo cropped by the platform however it feels like. This plan establishes a purpose-built shareable card image — a designed graphic generated from the card's data — and the authoring tool to create the overlay presets that drive it.

---

## Concept

Each shareable card image is a **1200×630 PNG** (standard OG dimensions) composed of:

1. **Hero photo** — fills the canvas. Defaults to the card's lead slideshow image. Swappable from the card's image pool.
2. **Text boxes** — freely positioned frames containing editable text. Each box is placed, resized, distorted, and transformed independently. Font style is set per box from a small named library.
3. **Logo slot** — a fixed position asset (placeholder now, real logo when it exists).

The combination of photo + text box layout is called an **OG Preset**. Presets are named, saved, and reusable across cards.

---

## Components

### 1. OG Maker (modal)

The authoring environment. Accessible in edit mode from the share flow or directly from an admin/settings surface.

**Canvas area**
- Renders at OG aspect ratio (1200×630), scaled to fit the modal
- Hero photo fills the canvas as a background
- Text boxes are rendered as absolutely positioned elements on top
- A text box can hang partially off any canvas edge — never fully off (at least one handle must remain reachable)
- `useDraggable` composable tracks position; `useTransformBox` composable tracks the box's transform state

**Adding a text box**
- Type content in an input field → hit "Add" → box drops onto the canvas sized to the text's natural proportions at the chosen style's default font size
- Box is immediately selected and ready to move/resize

**Selecting and transforming a box**
Clicking a box selects it and shows:
- **Corner handles** — drag to resize freely; text stretches/compresses to fill the box (horizontal and vertical distortion independent of each other)
- **Rotation handle** — above the top edge; drag to spin the box
- A small **toolbar** appears near the selected box with:
  - **Scale** — uniform resize (hold while dragging corner, or nudge with +/-)
  - **Shear** — horizontal skew slider; classic slant/italic effect independent of the font's own italic
  - **Style picker** — swap the box's font style key
  - **Edit text** — reopen the text input for this box
  - **Delete** — remove the box

**Text box style library**
A small set of named styles (OFL-licensed fonts only — safe to bundle and redistribute server-side):
- `POSTCARD` — all-caps condensed serif, large, high contrast, travel-postcard feel
- `SIGNAL` — all-caps sans, medium weight, tight tracking  
- `GHOST` — semi-transparent white, lighter weight, minimal footprint

Style defines font family and base weight only. Size and proportions are determined by the box dimensions — text always fills the box.

**Photo selector**
- Thumbnail strip of the card's image pool at the bottom of the modal
- Click to swap the hero photo — canvas updates immediately
- Logo slot visible as a positioned placeholder element

**Saving**
- Name the preset → Save
- Saved presets appear in the share flow picker
- Saving does not affect the card — presets are global, not per-card

---

### 2. Share flow integration

In the ShareModal (edit mode only):

1. **Photo picker** — defaults to lead slideshow image, swappable
2. **Preset picker** — grid of saved presets, each showing a small preview rendered with the current photo
3. **Adjust button** — opens OG Maker with the selected preset loaded and the current photo in place. Stamps can be nudged without altering the saved preset. The adjusted state is saved as `share_overlay_config` on the card.
4. **Done** — the configured state is persisted; `og:image` will reflect it on next unfurl

If no preset is selected, `og:image` falls back to the lead photo directly (no overlay).

---

### 3. Backend — PNG generation

**Route:** `GET /share/:id/card.png`

- Reads the card's `share_overlay_config` JSON
- Loads the specified photo from storage
- Composites photo + stamp overlays using the `image` crate
- Renders stamp text using `ab_glyph` (or similar)
- Returns PNG with appropriate cache headers
- **Fallback:** no config → serve lead photo directly (redirect or stream)

**Config shape (JSON stored on card):**
```json
{
  "photo_id": 42,
  "logo_visible": true,
  "text_boxes": [
    {
      "style": "POSTCARD",
      "content": "71 MACH ONE",
      "x": 0.08,
      "y": 0.72,
      "w": 0.45,
      "h": 0.14,
      "rotate_deg": 0,
      "shear_x": 0.0
    },
    {
      "style": "SIGNAL",
      "content": "MUSTANG",
      "x": 0.08,
      "y": 0.86,
      "w": 0.28,
      "h": 0.08,
      "rotate_deg": -3.5,
      "shear_x": 0.12
    }
  ]
}
```

All position and dimension values are fractions (0.0–1.0) relative to canvas size so they scale correctly to any output resolution. `rotate_deg` and `shear_x` are the only transform values stored — scale is implicit in `w`/`h`.

**Caching:** cache by `card_id + config_hash`. Invalidate when `share_overlay_config` changes.

---

## Data model changes

| What | Where | Change |
|---|---|---|
| `share_overlay_config` | `cards` table or card body JSON | New nullable JSON field — stores per-card adjusted preset config |
| `og_presets` | New table | `id`, `name`, `config JSON`, `created_at` |
| `share_photo_id` | Card body JSON | Which photo is selected as the share hero (nullable, falls back to lead) |

---

## Build order

> Architecture note: see `plan-og-overlay-single-renderer.md` for why the compositor is built before the editor UI, and why a `POST /share/preview` endpoint is needed alongside the public `GET /share/:id/card.png` route.

1. **Backend PNG route stub** — returns lead photo as redirect. Proves OG plumbing end-to-end before any UI exists.
2. **`og_presets` table + CRUD endpoints** — create, list, update, delete presets.
3. **Real compositor, minimally** — hero photo + at least one text box style (rotation + shear working), wired to both `GET /share/:id/card.png` and `POST /share/preview`. This is the source of truth; the editor is built to match it, not the other way around.
4. **OG Maker modal** — DOM layer handles interaction only (drag, handles, transform controls, style picker, photo swap). On drag-stop, calls `POST /share/preview` and swaps in the returned PNG as the canvas preview. Wire to preset save/load endpoints.
5. **Share flow integration** — preset picker + Adjust button in ShareModal. Reset-to-preset escape hatch.
6. **Full compositor polish** — remaining text box styles, edge cases, logo slot.

## Known risks and decisions

**Two-renderer risk** — The DOM canvas (CSS transforms) and the Rust compositor (glyph rendering) are two separate engines. The compositor is built first (step 2) and is the source of truth. The DOM canvas is explicitly an approximation — not a pixel-perfect match. A "compare" button that fetches the real PNG mid-session can be added to the OG Maker to let the author see the actual output during editing.

**Long content** — Each text box has a defined width (`w` fraction). Text always fills the box via distortion — it does not grow the box. No truncation. If content is too long and the result looks bad, the author resizes the box.

**Preset drift** — Adjusted per-card configs (`share_overlay_config`) are independent of the base preset. Improving a preset does not retroactively update cards that were adjusted from it. A "Reset to preset" button in the share flow clears `share_overlay_config` and re-applies the base preset.

**Font licensing** — All fonts in the style library must be OFL (SIL Open Font License) or equivalent. Google Fonts is the practical source. Choose fonts before coding the style library.

---

## Out of scope (for now)

- Per-user presets (global presets only for now; hook exists for later)
- Animated/video OG cards
- Client-side PNG export via Canvas API `toBlob()` — server compositor is the source of truth, but this is a valid fallback path if Rust text rendering proves too painful for complex transforms; revisit after step 2
- Reddit direct API (blocked by Reddit policy — pre-fill approach already in place)
- Discord webhook (separate item, not blocked on this)
