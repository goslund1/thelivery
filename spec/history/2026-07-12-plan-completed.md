# 2026-07-12: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Recently completed"

- **CarTabs shakedown + bug fixes**: spurious Springs and Dampers dialog on tab switch fixed (`_inPropUpdate` flag + `flush:'sync'` on gearCount watcher in TuningAdjustments — prevents prop-driven gearCount change from calling checkImplied); tune-name placeholder added to EditableText (CSS `::before` + `data-placeholder` attr) so empty tuneName in edit mode shows the car model name as hint text. — 2026-07-12
