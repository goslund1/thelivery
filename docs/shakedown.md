# Shakedown Checklist

Full manual test pass for thelivery. Run this before mobile, after major feature work, or any time something feels off. Check each item; note failures with a short description.

Dev server: `cd backend && cargo run` + `cd frontend && npm run dev`
Mobile: `npm run dev:mobile` — use Network URL in iPhone Safari.

---

## 1. Auth

- [ ] Visit `/ignition` or `/#ignition` → login modal opens, URL cleaned after
- [ ] Login with valid credentials → edit controls appear, Suggestions row visible in filters
- [ ] Login with bad password → error shown, no crash
- [ ] Logout → edit mode exits, admin UI hidden, JWT cleared
- [ ] With expired/invalid token → API call fails gracefully, re-login prompt appears

---

## 2. Edit mode enter / exit

- [ ] Click SideBug key icon while logged out → login modal opens (not edit mode)
- [ ] Click key icon while logged in → enters edit mode, body gets `editing-mode`, save buttons appear
- [ ] Exit with no dirty cards → exits immediately, no confirm
- [ ] Make an edit, click exit → ExitConfirmModal appears
- [ ] ExitConfirmModal → Save All → saves dirty cards, exits edit mode
- [ ] ExitConfirmModal → Discard → cards revert to pre-edit state, exits
- [ ] ExitConfirmModal → Cancel (×) → stays in edit mode, cards still dirty
- [ ] Escape while ExitConfirmModal open → cancels (stays in edit mode)

---

## 3. Inline edit

- [ ] Click any contenteditable field (card name, subtitle, section body) → editable
- [ ] Type → per-card dirty badge appears, unsaved count in EditBar increments
- [ ] Per-card Save button → saves just that card, dirty badge clears
- [ ] Dirty card → Discard exit → that card reverts

---

## 4. New card modal

- [ ] Open from EditBar + button → modal opens
- [ ] Fill name, subtitle, at least one collection → Save enabled
- [ ] Add tags → appear as chips
- [ ] Drag/drop photos → thumbnail strip populates
- [ ] Browse to add photos → same
- [ ] Click a thumbnail to set as feature image
- [ ] CarPicker → [+ FH5] or [+ FH6] → search → select → chip appears
- [ ] Tune name, share code (auto-formats to XXX XXX XXX), core specs
- [ ] Add upgrades via UpgradesPicker
- [ ] Add adjustments (sliders)
- [ ] Collapse a section in the modal → Save → card opens with that section collapsed
- [ ] Save → card appears in catalog with correct data
- [ ] Cancel → no card created, no orphan images left on server

---

## 5. Edit card modal

- [ ] Open via pencil icon in CardMeta (edit mode only)
- [ ] Name, subtitle editable and reflected on card on save
- [ ] Livery share code input → auto-formats, cursor stays at end
- [ ] Collections → add/remove chips
- [ ] Tags → add/remove chips, create new tag
- [ ] Inspiration / Design Notes textareas editable
- [ ] RecipeSection fully editable (tune name, share code, specs, upgrades, sliders)
- [ ] Collapse a section → Save → section starts collapsed in card view
- [ ] Cancel → all modal changes reverted (name, recipe, sections — everything)
- [ ] History button (top-right) → CardHistoryModal opens
- [ ] Delete card → confirm → card removed from catalog

---

## 6. Card history

- [ ] Open from EditCardModal top-right History button
- [ ] Version list appears, most recent first
- [ ] Select older version → diff displayed (changed sliders, upgrades, specs, text highlighted)
- [ ] Restore → card reverts to that version, modal closes
- [ ] Cancel → no change
- [ ] Escape → closes history modal (not EditCardModal)
- [ ] Second Escape → closes EditCardModal

---

## 7. Gallery / slideshow

- [ ] Card enters viewport → autoplay starts after settle delay
- [ ] Card exits viewport → autoplay suspends
- [ ] Manual pause (pause button) → stays paused across scroll in/out
- [ ] Resume → autoplay restarts
- [ ] Click thumbnail rail → stage updates, progress bar resets
- [ ] Thumbnail rail auto-scrolls to keep active thumb visible (no page jump)
- [ ] Click stage image → Lightbox opens full-size
- [ ] Lightbox prev/next navigation
- [ ] Lightbox Escape → closes lightbox only

---

## 8. Image management

- [ ] ImagePicker: upload photos via drag/drop and browse
- [ ] Reorder photos via drag
- [ ] ⤢ button on thumbnail → PhotoDetail opens (full-size, prev/next)
- [ ] PhotoDetail: alt text input saves to image
- [ ] PhotoDetail: CarPicker assigns car to individual photo
- [ ] Delete a photo in edit mode → photo removed from strip
- [ ] Save card → deleted photo file cleaned from server (orphan removal)

---

## 9. Tuning adjustments

- [ ] Sliders move, values update live
- [ ] Changed values highlighted (diff vs stock)
- [ ] Show Stock toggle → stock values shown in rows
- [ ] Define Stock → confirm dialog → stock values set to current
- [ ] Cmd+Z after Define Stock → undoes stock definition
- [ ] Gearing: locked slider → click/drag → transmission picker modal opens
- [ ] Transmission picker → select non-stock → rows unlock, upgrade auto-added to UpgradesPicker
- [ ] Return all gear sliders to stock → upgrade auto-removed
- [ ] Springs/Dampers: move alignment/springs/damping off stock → dialog fires once per session
- [ ] Suggest bar: appears in view mode → opens submit modal → fill fields → submit
- [ ] Suggest bar: × dismiss → hides for session
- [ ] Read-only mode (Suggestion Viewer): sliders visible but not interactive, suggest bar hidden

---

## 10. Recipe section gate

- [ ] Card with no tune name, share code, upgrades, or adjustments → recipe section hidden in view mode
- [ ] Same card in edit mode → recipe section always visible
- [ ] Add any recipe data → section appears in view mode

---

## 11. Suggestion viewer (admin only)

- [ ] Filters row shows "Suggestions" with count badge
- [ ] SideBug hamburger shows badge dot when count > 0
- [ ] Click Suggestions row → SuggestionViewer opens full-screen
- [ ] Pending tab: lists unreviewed suggestions
- [ ] Liked tab: lists liked suggestions
- [ ] Dropdown selects a suggestion → header updates (card name, make/model)
- [ ] Read-only sliders show diff vs card's current values
- [ ] Like button → suggestion moves to Liked, badge decrements, viewer advances
- [ ] Like again (on Liked tab) → toggles back to Pending, badge increments
- [ ] Dismiss → suggestion deleted, viewer advances to next; badge decrements if was pending
- [ ] Promote → new card created with suggestion adjustments, "(Updated)" name, share code cleared → EditCardModal opens
- [ ] Promote → save new card → appears in catalog
- [ ] Escape → closes EditCardModal (if open from Promote), second Escape → closes viewer

---

## 12. Filters & collections

- [ ] SideBug opens/closes filters flyout
- [ ] Collection checkboxes → cards filter in/out
- [ ] Favorites only → only starred cards visible
- [ ] Section expand/collapse checkboxes → all cards' sections respond
- [ ] Upgrades expanded checkbox → all TuningAdjustments expand upgrades tab

---

## 13. Theme builder

- [ ] Open via SideBug → Theme → Customize
- [ ] Base ambiance presets switch full theme
- [ ] Effects: glass opacity slider → panels update live
- [ ] Main palette: click color chip → ColorPicker wing opens
- [ ] ColorPicker: gradient + hue bar → pick color → theme var updates live
- [ ] ColorPicker: click palette swatch → anchors color; drag swatches to reorder
- [ ] Add swatch (+ button) → appears in palette
- [ ] Remove swatch (hover ×) → removed
- [ ] HSL namer: no swatch selected → live color name shown in title bar
- [ ] Close ThemeBuilder → changes persist
- [ ] Reload page → theme still applied

---

## 14. Card accent

- [ ] In EditCardModal, three color dots visible (Gold, Magenta, Blue)
- [ ] Click a dot → card accent color changes, cascades to sliders, chips, highlights
- [ ] Click same dot again → accent clears, returns to theme default
- [ ] × button clears accent
- [ ] Accent persists after save

---

## 15. Per-section defaultOpen

- [ ] Open EditCardModal → all sections expanded by default
- [ ] Collapse Inspiration section → Save → card shows Inspiration collapsed
- [ ] Reopen EditCardModal → Inspiration shows as collapsed
- [ ] Expand it → Save → card shows it expanded again
- [ ] Legend card sections behave the same

---

## 16. Escape key / modal stack

- [ ] Lightbox open → Escape → lightbox closes, nothing else
- [ ] ChipPicker open → Escape → closes
- [ ] ImagePicker open → Escape → closes
- [ ] History modal open → Escape → closes history, EditCardModal stays
- [ ] SuggestionViewer open + Promote EditCardModal on top → Escape → closes EditCardModal, viewer stays → second Escape → closes viewer
- [ ] LoginModal open → Escape → closes
- [ ] ExitConfirmModal open → Escape → cancels (stays in edit mode)

---

## 17. Mobile (run on device via `npm run dev:mobile`)

- [ ] Catalog loads on iPhone Safari
- [ ] Cards render correctly at narrow width
- [ ] SideBug opens/closes filters
- [ ] Slideshow autoplay works
- [ ] Lightbox opens, touch-nav works
- [ ] Edit mode accessible (login → key icon)
- [ ] EditCardModal usable at narrow width
- [ ] ThemeBuilder flyout usable at narrow width
- [ ] TuningAdjustments tabs scroll on narrow width
- [ ] No horizontal overflow / broken layouts

---

## Notes

**Shakedown run: 2026-07-05** (browser automation pass, desktop Chrome)

**Bugs found and fixed:**
1. `CardHistoryModal` z-index was 200 — sat under `EditCardModal` (1100). History button appeared to do nothing. Fixed: raised to 1150.
2. `EditCardModal` had no Escape handler when opened from `CardMeta` (inline Edit Card). Fixed: added `onKey` that calls `onCancel()` unless `historyCardId` is set (guards against closing both stacked modals at once).

**Verified working:**
- Auth: login/logout, invalid password error, edit controls appear on login
- Edit mode: enter/exit, ExitConfirmModal (Save All / Discard / Cancel), dirty tracking
- Inline edit: contenteditable, dirty badge, discard reverts
- EditCardModal: opens, HISTORY button, name/tags/collections editable, Cancel reverts, Escape closes
- CardHistoryModal: version list, Escape closes history only, second Escape closes EditCardModal
- Gallery/slideshow: autoplay, thumbnail rail, stage click → lightbox, Escape closes lightbox only
- SuggestionViewer: PENDING/LIKED tabs, diff display, read-only sliders, PROMOTE/DISMISS, Escape closes
- Filters: collection checkboxes, Favorites only, Show Sections, Suggestions row with badge
- ThemeBuilder: opens from SideBug → Customize, base ambiance presets, Effects sliders, Main Palette
- Suggest bar: visible in view mode with correct messaging

**Not run (requires device or manual interaction):**
- Section 4: New card full flow (photo upload, CarPicker, full RecipeSection)
- Section 8: ImagePicker drag/reorder, PhotoDetail alt text + CarPicker
- Section 9: Define Stock confirm dialog, gearing picker modal (locked slider)
- Section 14: Card accent — need edit mode + EditCardModal color dot clicks
- Section 15: Per-section defaultOpen toggle flow
- Section 17: Mobile (on-device only)

