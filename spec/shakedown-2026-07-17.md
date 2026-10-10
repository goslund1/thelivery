# Chrome Shakedown — 2026-07-17

Post-audit pass on localhost:5173 after code audit pass 4 (session 32).
Goal: confirm no obvious regressions across all major surface areas.

---

## View mode (unauthenticated)

- [ ] Gallery: slideshow auto-advances, manual advance on click
- [ ] Gallery: thumbnail strip nav jumps to correct image
- [ ] Lightbox: click main image opens lightbox
- [ ] Sections: INSPIRATION expand/collapse (text section)
- [ ] Sections: DESIGN NOTES expand/collapse (text section)
- [ ] Sections: TUNE / BUILD PARTS expand/collapse (recipe section)
- [ ] TuneTabs: click through a few category tabs (Tires, Gearing, Springs…)
- [ ] TuneTabs: View Inline opens inline view
- [ ] BUILD IT button visible and clickable
- [ ] Card title click → ShareModal opens
- [ ] ShareModal: Copy Link works (clipboard, 2s feedback)
- [ ] ShareModal: Post to Reddit opens pre-filled reddit.com tab
- [ ] Theme switcher: cycle through 2–3 themes, verify CSS swaps
- [ ] Filter/search flyout: opens and closes

## Auth + Edit mode

- [ ] Login modal opens (click edit icon or nav)
- [ ] Login succeeds, edit mode activates
- [ ] Edit a text field — card goes dirty
- [ ] Save card — dirty clears
- [ ] Exit with discard — changes rolled back

## OG Maker (auth required — the main new thing)

- [ ] Click card title → ShareModal → OG Designer section visible
- [ ] Open OG Maker modal
- [ ] Add a text box, type content — preview refreshes on each keystroke (the @input fix)
- [ ] Move/resize text box
- [ ] Toggle POSTCARD → SIGNAL → GHOST styles — compositor output updates
- [ ] Preset picker loads (auth header fix from this session)

---

## Log

| # | Item | Result | Notes |
|---|------|--------|-------|
| 1 | Gallery: slideshow autoplays | ✓ PASS | Autoplays on load |
| 2 | Gallery: click pauses/advances | ✓ PASS | Shows PAUSED indicator |
| 3 | Gallery: thumbnail nav | ✓ PASS | Clicking thumbnail changes main image |
| 4 | Lightbox: ⤢ hover button | ✓ PASS | Appears on hover (top-right of image); click = not lightbox — lightbox is the ⤢ button only |
| 5 | Lightbox: opens with nav/download/close | ✓ PASS | 1/14 counter, ‹ › nav, ↓ DOWNLOAD, Escape closes |
| 6 | Sections: text sections expand/collapse | ✓ PASS | INSPIRATION and DESIGN NOTES toggle cleanly |
| 7 | Sections: TUNE/BUILD PARTS expands | ✓ PASS | Full content: core specs, upgrades, adjustments |
| 8 | CarTabs: multi-car tab switching | ✓ PASS | 3 tabs (599D, Corvette, Sprite); gallery filters per car |
| 9 | BUILD IT button | ✓ PASS | Scrolls to recipe anchor (#recipe-1) |
| 10 | TuneTabs: View Inline toggle | ✓ PASS | Button switches to "View As Tabs" and back |
| 11 | Card title → ShareModal | ✓ PASS | Share URL, Copy Link, POST TO REDDIT, Discord stub, OG section |
| 12 | OG Maker: opens from ShareModal | ✓ PASS | Start Blank opens modal with photo and text controls |
| 13 | OG Maker: live preview on keystroke | ✓ PASS | @input fix confirmed — compositor renders POSTCARD text within ~200ms |
| 14 | OG Maker: style toggle (SIGNAL) | ✓ PASS | Dark chyron backdrop renders correctly |
| 15 | Theme switcher: picker opens | ✓ PASS | All 5 themes + Customize listed |
| 16 | Theme switcher: Rainbow applies | ✓ PASS | Diagonal stripe border appears instantly; CSS swap confirmed |
| 17 | Filter flyout | ✓ PASS | Section toggles, collection checkboxes, color chips, Favorites only |
| 18 | Auth: session persists | ✓ PASS | Already logged in as Jason; account modal shows correctly |
| 19 | Edit mode: activates | ✓ PASS | Editable fields, tag chips, color swatches, EDIT bar all appear |
| 20 | Dirty tracking | ✓ PASS | "Unsaved Changes" modal fires on exit with dirty field |
| 21 | Discard and exit | ✓ PASS | Original values restored, edit mode exited cleanly |
| — | Console errors (app) | ✓ CLEAN | 1 error from Claude-in-Chrome extension only; zero app errors |
