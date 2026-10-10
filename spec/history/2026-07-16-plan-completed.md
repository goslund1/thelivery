# 2026-07-16: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Recently completed"

- **Code audit pass 4** — six fixes across compositor, OG Maker, ShareModal, auth: `@input` on content field for live preview; scrim overlap check; `as any` removed from ShareModal; `list_og_presets` auth-gated; `render_glyphs` + `blit_with_transform` extracted (−75 lines); `TextStyle` enum + `OgTextStyle` union replace magic strings. Deferred: circular store import warning (`ui.ts` ↔ `modal.ts`). — 2026-07-16 (commit b3214b2, AAR: `docs/aar-2026-07-16b.md`)

- **OG overlay access fixed** — ShareModal OG design section now gated by `auth.isAuthenticated` instead of `ui.isEditing`. Admins see the overlay composer in normal user view (click card title → Share modal) — the correct flow since sharing is a post-edit action, and the title is an editable input in edit mode. Also added `/share` to the Vite dev proxy so the compositor preview and share routes work in local dev. — 2026-07-16

- **OG Overlay Studio** — server-generated 1200×630 `og:image` for social share unfurls (Discord, iMessage, Reddit). Pure-Rust compositor (`fontdue` + `image` crate): `POST /share/preview` for live OG Maker preview, `GET /share/:id/card.png` for public card image, `og_presets` table + CRUD. Three text styles: POSTCARD (Bebas Neue, floating), SIGNAL (Oswald VF + dark chyron backdrop), GHOST (Oswald VF, semi-transparent). Polish: bottom scrim gradient, per-style drop shadows, logo slot ("THE LIVERY" placeholder). OG Maker modal: freely-positioned text boxes with corner-resize, rotation handle, shear slider; 200ms debounced live preview; preset save + "Save to Card." ShareModal integration with preset picker, preview thumbnails, Adjust/Reset flow. — 2026-07-16
