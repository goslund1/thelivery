# 2026-07-07: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Recently completed"

- Settings/Admin UI reorganization: Account panel (change password + Add User gated behind toggles, Admin Panel → button, Sign Out); separate Admin panel with Tools tab (image tools, stats, orphans, trash, seed) + Export Card tab (YAML download/import, legacy repair at bottom); Tune Suggestions stays in Filters flyout only — 2026-07-07

- figurePath patching in migration flow: ImageMigrationModal snapshots old paths and patches TextSection.figurePath after migrate; Repair Figure Paths endpoint matches by sequence number before falling back to lead image — 2026-07-07
