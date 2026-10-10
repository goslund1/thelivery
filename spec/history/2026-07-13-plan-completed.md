# 2026-07-13: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Recently completed"

- **CarTabs/TuneTabs shakedown + polish**: discipline baseline presets (Race/Rally/Drift/Street) seeded in DB; `+ ADD CAR` / `+ ADD TUNE` buttons styled as dashed tabs (grey→gold/pink on hover); TuneTabs shelf color = `--highlight`; spacing and gap fixes; `v-tip-up` directive for right-flying tooltip from text end; card title opens ShareModal. — 2026-07-13

- **Social sharing foundation + Reddit pre-fill**: OG share page endpoint (`GET /share/:id/:slug`) added to Rust backend — server-rendered HTML with full OG + Twitter card tags; real browsers get `<meta refresh>` to `/`. Card title opens `ShareModal` (view mode only, `v-tip-up` tooltip). Modal: Copy Link button (clipboard API, 2s feedback) + **Post to Reddit** (pre-fill approach — opens `reddit.com/submit?url=…&title=…` in new tab with editable title pre-populated from card + car name + share code; Reddit's native form handles subreddit selection and posting) + Discord stub. Note: direct Reddit API posting blocked — Reddit ended self-service OAuth access Nov 2025 for personal projects. See `docs/plan-reddit-share.md`. — 2026-07-13
