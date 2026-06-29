# Touchups & Bug Fixes Log

Running record of symptoms, root causes, and fixes. Append new entries at the bottom.

---

## 2026-06-27/28 — Session 2

### Credential loss on `rm data.db`
**Symptom:** Deleting the database wiped the user account; `adduser` + backend restart then invalidated the JWT because the secret regenerated.
**Root cause:** No persistent credentials and no persistent JWT secret.
**Fix:** Created `backend/.env` with a stable `JWT_SECRET`. Created `backend/seed/users.json` (gitignored) loaded by `seed_users_if_empty()` on startup — `rm data.db` + restart now restores the user automatically.

---

### Save-and-exit stuck in edit mode (legend card)
**Symptom:** Clicking save → "save and exit" dialog → confirming → still in edit mode.
**Root cause:** Backend returned 403 for any PUT to the legend card (catalog_number 0). `saveAllDirty` threw, so `isEditing` never flipped false.
**Fix:** Removed the hardcoded `catalog_number == 0` guard from `put_card` in `main.rs`. The `LegendConfirmModal` already protects saves at the UI level.

---

### Stacked view broken after adding static/variable toggle
**Symptom:** The "stacked" card layout showed garbage rows / blank entries.
**Root cause:** Sentinel rows (`key: '__mode_<tabId>'`) had a valid `tab` string field, so `storedRows` included them. They have no display data so they rendered as blank rows.
**Fix:** Added `&& !r.key.startsWith('__mode_')` to the `storedRows` computed filter in `TuningAdjustments.vue`.

---

### ARB sliders showing wrong decimal places
**Symptom:** ARB values displayed as `41.1` instead of `41.10` — trailing zero dropped.
**Root cause:** Step was set to `0.1`, so `decimals(r)` returned 1. Should be 2 decimal places.
**Fix:** Changed ARB step to `0.01`. `decimals(r)` now returns 2 for any `step < 0.1`.

---

### Up/down arrow keys moved slider value instead of navigating rows
**Symptom:** Arrow up/down nudged the slider thumb; no way to move between rows by keyboard.
**Root cause:** All four arrow keys were wired to value adjustment.
**Fix:** In `onRowKeydown`, up/down now focus the previous/next `.ta-slider[data-key]` element; left/right still adjust value. Each slider got a `data-key` attribute to support the querySelector lookup.

---

### First click on slider row moved the thumb
**Symptom:** Trying to select a row to work on inadvertently moved the value.
**Root cause:** `mousedown` on the range input immediately starts drag interaction.
**Fix:** Added `onSliderMouseDown` — if the clicked row isn't already focused, `preventDefault()` and set focus only; second click proceeds normally.

---

### Unit symbols (%, °) not inside min/max edit fields
**Symptom:** Min/max fields for brakes and diff showed raw numbers; `%` appeared outside or not at all.
**Root cause:** Fields were plain number inputs; unit was only shown in the slider label.
**Fix:** Min/max fields are now `type="text"`. Display value is formatted via `fmt(r, val)` which appends `r.unit`. `endDisplay` tracks the raw user-typed string during editing to avoid cursor/caret issues.

---

### Slider thumb couldn't be dragged after stock dot border was added
**Symptom:** Adding a conditional border to the stock dot (`.ta-stock-tick`) made the slider thumb unresponsive to drag.
**Root cause:** `.ta-stock-tick` had `z-index: 2` and was absorbing pointer events, sitting above the slider input in hit-test order.
**Fix:** Added `position: relative; z-index: 3` to `.ta-slider` so the input sits above the stock dot in stacking order. Added `box-sizing: border-box` to `.ta-stock-tick` so the border doesn't resize the dot.

---

### Broken image in legend card inspiration section
**Symptom:** Inspiration section showed a broken image placeholder (?) on the legend card.
**Root cause:** `figurePath` in the DB still pointed to `/uploads/legend-inspiration.png`, which was deleted when the database was wiped.
**Fix:** Cleared the stale `figurePath` directly in SQLite via Python (`sqlite3` library). The section now shows the empty "Select image" state, ready for a new upload.

---

### Fade/shadow on add-images button shifts left ~16px on slide change
**Symptom:** The gradient fade on the left edge of the add-images button panel appeared to jump ~16px left whenever the slideshow advanced to a new slide.
**Root cause:** The `.edge-arrow-right` element has its own `linear-gradient` background. In edit mode it sits at `right: 70px`, making it 36px wide and extending to `right: 106px` — 16px past the left edge of the add panel's fade (`right: 90px`). When `canRight` flipped true after a slide change, the two overlapping gradients combined to push the visible fade edge leftward.
**Fix:** Added `background: none` to `.thumb-rail-editing .edge-arrow-right` in `Gallery.vue`. The add panel's gradient is the sole source of the fade; the arrow just renders its chevron triangle on top.

---

### First-click row select moved slider thumb; fix blocked trackpad drag gestures
**Symptom:** Clicking an unselected slider row to focus it accidentally moved the thumb to the click position. After that was guarded, directly clicking and holding the thumb dot still required a prior row-select click — it wouldn't grab in one motion.
**Root cause:** The guard needed to distinguish between two intents that arrive as the same event: a track click (select only) vs a thumb click (select + grab).

**Tried first — `e.preventDefault()` on all first-click mousedowns:** Prevented accidental track moves. This is a reasonable approach when the element is simple and uniform — if you just need to block an entire interaction zone on first click, it's clean and straightforward. Didn't fit here because this slider needs to support OS-level trackpad drag gestures (double-tap-to-drag, click-hold-drag), which require the first `mousedown` to go unblocked so the OS can set up its gesture state. In a mouse-only or non-gesture context this would have been fine.

**Tried second — `pointer-events: none` on unfocused slider:** Clicks fell through to the row div, focused cleanly, and OS gestures worked. This is a solid pattern for blocking an element entirely until it's activated — simple, no JS event logic needed. Didn't fit here because the slider has two sub-zones with different intent (track = select only, thumb = select + grab), and `pointer-events` applies to the whole element with no way to differentiate. In a case where the whole element should be inert until focused, this would be the right call.

**Fix:** In `onSliderMouseDown`, calculate whether the click landed within 12px of the thumb center using the same `7 + p * (width - 14)` formula as the thumb renderer. Track clicks on an unfocused row call `e.preventDefault()` and just select. Thumb clicks skip the prevent and grab immediately — focus + drag in one motion. ✓ Confirmed working.

---

### TuningAdjustments crash on every page load bricks login and event system
**Symptom:** After any page reload, the app appears to load normally but login hangs at "Signing in…", the stoplight overlay is missing, and nothing is clickable — a full brick requiring another reload.
**Root cause:** The legend card uses `v-show` (not `v-if`), so `TuningAdjustments` mounts on every page load before card data arrives from the backend. At that first render `props.adjustments` is a sparse array with an undefined slot. `initTabModes` and the `storedRows` computed both called `r.key.startsWith(...)` with no null guard — throwing `TypeError: Cannot read properties of undefined (reading 'startsWith')` during Vue's setup phase. This crashed Vue's internal event/scheduler loop for the whole session, preventing login responses from being processed.
**Fix:** Added optional chaining in both places: `r?.key?.startsWith('__mode_')` and `r?.key &&` guard in the `storedRows` filter. Undefined slots are now silently skipped.

---

### FactoidPanel invisible but blocking all clicks on right side of page
**Symptom:** User can't click anything on the right ~320px of the page — settings icon, stoplight, card controls — all unresponsive. No visible cause.
**Root cause:** `.fp-panel` is always in the DOM (only the backdrop uses `v-if`). When the panel is "closed" it's off-screen via `transform: translateX(100%)`, but it still has `z-index: 200` and `pointer-events: auto`, so clicks on the right edge of the viewport hit the panel silently. If a page reload or HMR glitch leaves `factoidPanelOpen = true` in the store, the panel sits fully on-screen in dark-theme colours that blend into the background — completely invisible but eating every click.
**Fix:** Added `:style="{ pointerEvents: ui.factoidPanelOpen ? 'auto' : 'none' }"` to `.fp-panel` so the off-screen panel never intercepts input.

---

### Slider rows evolved from a single flat layout to a two-section structural model
**Context:** Design evolution, not a bug fix. Documents a deliberate architectural shift in how tuning adjustment rows are composed.
**Starting point:** Each slider row was a single flat flex container — label, value box, min field, slider track, and max field all as siblings in one row. Visual groupings (grey background for the label/value zone, magenta title bars) were painted on using CSS gradients and absolute positioning. Changing the width of the label zone had no effect on the slider because the layout wasn't actually divided — it was cosmetic.
**Problem that forced the change:** When widening the left "label and output values" area via a CSS variable, the gradient stop moved but the flex items didn't. The slider bars were reintroduced into the grey zone rather than being pushed out. Patching the gradient wasn't structural — it couldn't make the two zones genuinely respond to each other.
**Evolution:** Recognised that what looked like one row was conceptually two distinct modules: (1) the label and current value output, and (2) the slider with its min/max range controls. These have different visual weight, different interaction roles, and different scaling needs.
**Fix:** Refactored every row and group header to use two real flex children — `.ta-left-section` (fixed width, `var(--col-left-zone)`, grey background) and `.ta-right-section` (flex:1, slider zone). The title bars above each group already used this two-zone pattern (magenta left, grey right), so the rows now match structurally. Adjusting `--col-left-zone` now genuinely compresses or expands the slider. The group's `overflow: hidden` clips the grey column at the bottom corner cleanly.
**Why this approach over a full component split:** A full modularisation of the slider into its own component would have cost more time re-establishing the visual fidelity than was worth it at this stage. The two-section wrapper achieves the same conceptual separation with minimal disruption — the slider rows are now structured so they could be extracted into a standalone module later without a full rethink.

---

### Active thumbnail could land partially under the add-images panel
**Symptom:** When the slideshow advanced to a later slide, the active thumbnail's highlight ring appeared partially obscured by the add panel rather than in a fully visible slot.
**Root cause:** The `watch(index)` scroll logic used `t.clientWidth` as the full right boundary, not accounting for the 90px panel that covers the right side of the rail.
**Fix:** In edit mode, `reservedRight = 90` is subtracted from the effective right boundary before checking/setting `scrollLeft`. The active thumb now always lands in the clear zone; the next thumb peeks through the gradient as a preview, and the last slide leaves empty space before looping.
