# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.
Completed items move to `docs/completed/`.

---

## Active — ordered by priority

### 1. TuningAdjustments — gate section on recipe / adjustments existing
- `<TuningAdjustments>` and its "Tune Adjustments" label render unconditionally in RecipeSection.
- Should be hidden on cards with no relevant data (e.g. photo safari cards, cards with no car IDs, cards with no adjustments in non-edit mode).
- Gate condition: hide when `local.adjustments` is empty AND not in edit mode.
- Note: multi-car mashup cards (parked) will eventually need a per-car tuning view; the gate logic here should not assume a single carId — defer that complexity to the mashup plan doc.

### 2. Card accent override (per-card color)
- Optional `accentOverride` field on Card: `inherit | gold | magenta | custom`.
- Edit-only affordance in CardMeta (+ EditCardModal / NewCardModal for parity).
- No backend work — rides existing card JSON.
- Drift cards in collections → suggest magenta as default.
- Note: CSS already uses `--accent`, not `--gold`; the concept is valid, variable names in old spec were stale.

### 3. Battle-test checklist
- Once the main items above are done, a focused pass to verify all dialog interactions, edge cases, and flow completeness. Includes:
  - Gearing dialog round-trip (locked slider → modal → transmission pick → rows unlock → UpgradesPicker reflects → stock restore → upgrade removes)
  - All confirm/cancel dialogs
  - Edit mode enter/exit with dirty cards
  - New card modal full flow
  - History restore
  - Theme switching with ThemeBuilder open
  - Add more as we think of them

---

## Parked

### Submit Tune feature
- Full background at `docs/plan-submit-tune.md`.
- Trigger redesign is the first problem: current logic fires on first non-stock move but cards ship with non-stock values.
- Before building: write a finishing-moves plan doc covering trigger logic, contact/credit form (Gamertag, PSN, Discord, Reddit, etc.), "Ask Me Later" state, backend submissions table + API, admin queue ("the pile").

### Mobile layout
- Theme builder flyout + general catalog narrow-screen pass.
- Deferred until active list is cleared.

### Multi-car mashup card (plan doc needed first)
- Discussed: single card with multiple carIds, each tied to specific photos; user clicks photo or make/model group to surface that car's tune + parts.
- Low priority. Write a plan doc before touching any code — the data model changes are non-trivial.

### Design decisions (need a conversation)
- Badge indicators: Abide/Victory/Smokin' as known tag values (reuse ChipPicker/TagCloud) vs dedicated field with icon rendering.
- Per-section default-expanded: does the global toggle cover it, or do we need per-section "open by default"?

---

## Maintenance

### Backfill pass (another round coming)
- Card data was brought in line once. Expect another round after card accent, tuning gate, and other structural changes land.
- Done in-app via EditCardModal; revisit when the dust settles.

---

## Recently completed

- TuningAdjustments — gearing-only lock/unlock, navigation scope fix, tire pressure unlock — 2026-07-05
- ThemeBuilder effects sliders: picker opacity + color swatches for both glass surfaces — 2026-07-04
- TuningAdjustments — Define Stock confirm dialog + in-session Cmd+Z undo snapshot — 2026-07-04
- All 5 ta-prompt-strip dialogs converted to centered glass modal (ta-trans-modal-backdrop) — 2026-07-04
- CLAUDE.md + docs/plan.md housekeeping, session AARs — 2026-07-04
- Transmission/gearing system (locked sliders, glass picker modal, auto-add/remove upgrades, view+edit parity, legacy name migration) — 2026-07-04
- Car identity: `cars` table, CarPicker, PhotoDetail, alt text — 2026-07-04
- Suggest bar two-surface redesign — 2026-07-04
- Theme builder + ColorPicker (palette drag-reorder, title bar, HSL namer, DrawerPanel) — ~2026-06-30
- Card history UI (CardHistoryModal) — earlier session
- Card migration tool (YAML export/import) — earlier session
- CSS variable rename: `--gold` → `--accent` — earlier session
- Add-photos SVG icon (thumbs rail add button) — resolved
- Initial backfill of card data fields — done, another round expected
