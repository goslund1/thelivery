# 2026-07-10: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Recently completed"

- Replaced `vue-virtual-scroller` with plain `v-for` (all filtered cards always mounted). Virtual scroll's slot-pool model is incompatible with stateful Vue components — two failure modes (slot recycling + concurrent duplicate slots) required module-level singleton workarounds that added permanent complexity. At catalog scale the memory cost of mounting all cards is negligible. `useCardVisibility.ts` (IntersectionObserver + KeepAlive) left in place for when the catalog is large enough to need lazy mounting. See `CLAUDE.md` → "Card list rendering" — 2026-07-10
