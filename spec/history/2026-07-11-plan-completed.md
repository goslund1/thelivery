# 2026-07-11: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Recently completed"

- **Gear preset fix + preset kind** (migration 0016): `applyPresetValues`, `executeApplyPreset`, and `saveAsPreset` all now include locked gear rows — previously skipped them, so presets saved/applied without Race Transmission had no gear values. `getAdjustments()` now includes locked rows so gear values survive the flush→rebuild cycle. `applyImpliedTransmission()` added: infers required transmission from highest `gearN` key in preset, auto-adds it via `implied-upgrades` emit (mirrors `onTransChoice` pattern). Rally preset (id=6) was saved while locked and has no gear keys — needs a re-save after adding Race Transmission. Migration 0016 adds `kind TEXT NOT NULL DEFAULT 'build'` to `tuning_presets`; Baseline checkbox in save dialog; `◆` prefix on baseline entries in dropdown. Baseline/build conflict logic (skip baseline-locked rows when applying a build preset) deferred until first real baseline preset exists. — 2026-07-11

- **RefImg pipeline** (migration 0015): `image_role` + `included` columns on images table; `included` now persists across page reloads (was ephemeral). RefImg images get structured filenames `{card_slug}_RefImg{NN}_{date}_{uuid6}`, default to excluded from slideshow, skip livery creation + color assess. CarPicker gets `+IMG` button (`showImageBtn` prop, dashed style) that emits `select-image` — no car search. NewCardModal tracks `imageRole`, shows RefImg chip, passes role to uploads. `sync_card_images` now syncs `included` back to DB on card save. — 2026-07-11

- **NewCardModal car/recipe parity**: `RecipeSection` gets `forceEdit` prop so all edit-mode fields (CarPicker, spec table, upgrades, preset bar) appear in NewCardModal without requiring global edit mode. — 2026-07-11
