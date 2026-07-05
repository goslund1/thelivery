# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.
Completed items move to `docs/completed/`.

---

## Active — ordered by priority

### 1. Battle-test checklist
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

### Mobile layout
- Theme builder flyout + general catalog narrow-screen pass.
- Deferred until active list is cleared.

### Multi-car mashup card (plan doc: `docs/plan-multi-car-mashup.md`)
- Single card with multiple car variants, each with its own photos, tune, upgrades, and adjustments.
- Tab strip inside RecipeSection switches between cars; gallery filters to match.
- Foundation: `images` table migration + CarPicker at upload time; variants array on ForzaRecipeSection.
- Single-car cards untouched — additive change.

---

## Maintenance


### Backfill pass (another round coming)
- Card data was brought in line once. Expect another round after card accent, tuning gate, and other structural changes land.
- Done in-app via EditCardModal; revisit when the dust settles.

---

## Recently completed

- Suggestion Viewer + Promote: SuggestionViewer.vue shadowbox, Pending/Liked tabs, read-only TuningAdjustments with diff highlighting, Like/Dismiss/Promote actions, SideBug badge + Filters row entry; Promote clones source card with suggestion adjustments (share code cleared, "(Updated)" name), opens in EditCardModal immediately — 2026-07-05
- Per-section default collapsed/expanded: `defaultOpen` on section types, set at modal save time, seeds CardView openState — 2026-07-05
- Card accent override: `accentOverride` field on Card, three preset color dots in CardMeta edit mode — 2026-07-05
- RecipeSection one-way data flow refactor: replaced loop-prevention flags with resetToken prop — 2026-07-05
- Recipe section gate: hide Tune / Build Parts bar in view mode when tuneName, shareCode, upgrades, and adjustments are all empty — 2026-07-05
- Code quality pass 2: formatShareCode dedup in EditCardModal, querySelector<HTMLElement> typing, mutateDirty Set helper, closeTopModal() priority-ordered Escape handling (fixed 3 missing modals), installedParts computed cache in RecipeSection — 2026-07-05
- Code quality pass 1: formatShareCode util, collectOrphans helper, watcher consolidation, String() coercion removal — 2026-07-05
- TuningAdjustments — gearing-only lock/unlock, navigation scope fix, tire pressure unlock — 2026-07-05
- ThemeBuilder effects sliders: picker opacity + color swatches for both glass surfaces — 2026-07-04
- TuningAdjustments — Define Stock confirm dialog + in-session Cmd+Z undo snapshot — 2026-07-04
- All 5 ta-prompt-strip dialogs converted to centered glass modal (ta-trans-modal-backdrop) — 2026-07-04
- CLAUDE.md + docs/plan.md housekeeping, session AARs — 2026-07-04
- Transmission/gearing system (locked sliders, glass picker modal, auto-add/remove upgrades, view+edit parity, legacy name migration) — 2026-07-04
- Car identity: `cars` table, CarPicker, PhotoDetail, alt text — 2026-07-04
- Submit Tune feature: suggest bar, submit modal, backend submissions table + admin queue — shipped earlier, plan entry was stale
- Suggest bar two-surface redesign — 2026-07-04
- Theme builder + ColorPicker (palette drag-reorder, title bar, HSL namer, DrawerPanel) — ~2026-06-30
- Card history UI (CardHistoryModal) — earlier session
- Card migration tool (YAML export/import) — earlier session
- CSS variable rename: `--gold` → `--accent` — earlier session
- Add-photos SVG icon (thumbs rail add button) — resolved
- Initial backfill of card data fields — done, another round expected
