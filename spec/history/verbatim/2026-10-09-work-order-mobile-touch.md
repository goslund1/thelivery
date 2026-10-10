# The Livery — Mobile & Touch Work Order

Oct 9, 2026 · @Jason Hunter

This work order makes thelivery a working tool on a phone, with entering a tune from the Xbox screen as the headline job, while the desktop layout stays as it is. It is written for Claude Code to execute one phase at a time. Phases 0 to 2 are short and exist to unblock Phase 3, the tuning panel, which is the priority.

## Scope

The phone is a full tuning-entry and viewing surface; site administration stays on desktop. "Phone" means viewport width ≤ 640px; "Touch" means any device without hover, including iPad.

| Surface | On a phone | Status |
| --- | --- | --- |
| Browse: cards, slideshow, lightbox, filters, favourites, text size, theme pick, share link | Full | Settled |
| Tuning: sliders, typed values, tabs, presets, gearing/transmission dialog, save | Full, first-class. This is the headline job. | Settled |
| Card images | Edge to edge, no gutter (or a few pixels) | Settled |
| Slideshow building | Full: importing, arranging and naming photos, choosing what is in the slideshow, the lead image, and playback | Settled |
| Build parts: car picker, spec selects, upgrades picker | Full; the section must work on a phone screen | Settled |
| Card text: title, subtitle factoids, share codes, tags, collections; New Card | Full. Expected to need little beyond the gutter and touch fixes; see E5 | Settled |
| Theme Builder (custom colours) | Desktop browser only. Theme switching stays everywhere. | Settled |
| OG Maker | Hidden on phones, kept on iPad and desktop. It is the share-card designer: opened from a card's Share dialog when signed in, it lays text boxes over a card photo (font, style, move, resize, rotate) to make the 1200×630 preview image that Discord, iMessage and Reddit show when the card's link is pasted. Dragging boxes on a canvas that wide is poor at phone size. | Settled |
| Admin panel, users, image migration, trash, seed, suggestion moderation, factoid types, card history, YAML export | Not optimised. Must still open and close without breaking. | Settled |

## Ground rules for CC

Every change hangs off one of two independent axes, and the desktop view must not move.

**Two axes, never one**

- **Layout follows width.** Phone layout applies at `max-width: 640px` (the breakpoint `catalog.css` already uses). The existing 540px and 800px upgrade-column breakpoints stay.
- **Interaction follows capability.** Touch behaviour applies under `@media (hover: none), (pointer: coarse)`. This is what covers iPad.
- No user-agent sniffing and no `isMobile` flag driving styles. iPadOS Safari reports itself as a Mac.

**Desktop parity**

- At 1440×900 with a mouse, before and after screenshots must match. Phase 0 captures the baseline.
- Any change that would alter desktop is listed in the phase plan and approved before it is made.

**Where the CSS goes**

- `src/styles/catalog.css` keeps its class names and existing rules, per `CLAUDE.md`.
- New global overrides go in a new `src/styles/responsive.css`, imported in `main.ts` directly after `catalog.css`.
- Components with scoped styles get their media queries inside the same file.

**Process**

- Open each phase in plan mode: state the approach and the files, then wait for a go-ahead.
- Name the standard pattern being used (collapsible edge drawer, sticky tab bar, range input on touch) before writing code. Do not invent one.
- Reproduce each item at its stated viewport first. This audit ran on GitHub commit `3c6709c` (2026-07-17); if the local copy is ahead and an item no longer reproduces, say so and skip it.
- One phase per commit. Never auto-commit. Run `npm --prefix frontend run build` after each phase.
- Unrelated problems found along the way go into `docs/plan.md`, not into this work.
- An item is done when its "Done when" check passes in a screenshot or a measurement. Report anything unverified as unverified.

## Phase 0 — Harness and baseline

Build the measuring tool first, so every later phase is checked the same way and desktop drift is caught.

| ID | Change | Done when |
| --- | --- | --- |
| H1 | Extend `frontend/shot.mjs` to take a viewport list with a touch flag: 360×640, 390×844, 844×390, 820×1180 and 1180×820 as touch; 1440×900 as mouse. | One command writes a labelled set of screenshots per viewport. |
| H2 | Per viewport, capture: page top, first card collapsed, first card with all sections expanded, the tuning panel, and the same card in edit mode. | All five views exist for all six viewports. |
| H3 | Add an overflow audit to the script: report when the document scrolls sideways, and list any element inside `.card` whose content is wider than its box, apart from `.thumbs` and `.ta-tabbar`. | The audit prints the known failures from this document before any fix. |
| H4 | Add a touch audit: list in-scope controls under 44px in either dimension and text fields under 16px. | Same. |
| H5 | Save the untouched "before" set to `docs/mobile-pass/before/`. | The 1440×900 set is the desktop parity reference. |

## Phase 1 — Layout

At phone width the card runs edge to edge and nothing clips; these are all width rules in `responsive.css` unless a file is named.

| ID | Change | Done when |
| --- | --- | --- |
| L1 | **Edge-to-edge cards.** At ≤ 640px: `body` side padding 0 and top padding 12px; `.card` and `.page-head` lose side borders and radius; `.catalog` gap 16px. `.stage` spans the full viewport width. | At 390px the stage image measures 390px wide. |
| L2 | **Text padding.** At ≤ 640px, inner padding drops from 24px to 14px on `.card-meta`, `.tag-cloud`, `.thumbs`, `.section summary`, `.section-body`, `.nc-section-head`, `.nc-footer`. | Text column is at least 332px wide at 360px. |
| L3 | **Remove fixed minimums.** `.gutter-text` and `.anecdote-text` lose `min-width: 296px`; `.gutter-figure` takes full width. | At 360px no paragraph is cut off at the right edge. |
| L4 | **Spec table.** `.recipe-table` cannot fit five columns of labels. At ≤ 640px render it as label/value rows, one spec per line. | "Drivetrain", "Transmission" and "Suspension" read in full at 360px. |
| L5 | **Type scale.** At ≤ 640px: `.card-title` 42px → `clamp(28px, 8.5vw, 42px)`; `.tune-name` 21px → 18px. | "Tune In Tokyo" fits one line at 390px. |
| L6 | **Side bug.** Below 1067px wide it sits on top of the card's right edge (`positionSideBug()` in `SideBug.vue` clamps it to `right: 0`). At ≤ 640px it starts collapsed to a single tab on the right edge, 44px tall; tapping the tab opens the full bug floating over the card as it does now, and an outside tap collapses it (decision D1). Reuse the tab-and-wing language of `DrawerPanel.vue`. No persistent control is pinned to the bottom edge on phones. From 641px to 1066px the bug stays open and `body` reserves right padding equal to its width. | At 390px the collapsed tab covers at most 24px of the card edge. At 820 and 1024px no card pixel sits under the bug. 1440px unchanged. |
| L7 | **Viewport units and safe areas.** Add `viewport-fit=cover` to the viewport meta in `index.html`. Fixed bottom elements add `env(safe-area-inset-bottom)`. Every `vh` max-height on a fixed panel gains a `dvh` line after it (10 files use `vh`). | On the iPhone nothing fixed sits under the home indicator or Safari's toolbar. |

## Phase 2 — Touch foundation

These are capability rules, so they apply to phones and iPads alike and leave mouse behaviour alone.

| ID | Change | Done when |
| --- | --- | --- |
| T1 | **Tooltips.** `vTip` and `vTipUp` in `composables/tooltip.ts` attach their `mouseenter` listeners only when `matchMedia('(hover: hover)')` matches. Today a tap opens the tip and it stays open over the flyout. | Tapping each side-bug button leaves `.custom-tip` at width 0. |
| T2 | **Hover-only controls.** On touch, show without hover: `.stage-expand`, `.thumb-controls` (edit mode), `.nc-thumb-x`, `.nc-thumb-star`. | Each has computed opacity 1 and is tappable on first touch. |
| T3 | **Tap targets.** In-scope controls get a 44×44px hit area on touch, by padding or an inset `::after`, keeping the drawn size where the look matters. Worst today: `.chip-remove` 7×12, `.filter-color-chip` 18 tall, `.bug-check input` 14, `.accent-chip` 14, `.thumb-ctrl` 19, `.up-step-btn` 20, `.kit-stock-btn` 17, `.ta-tab-btn` 27 tall, close buttons 12–21 wide. | The H4 audit lists no in-scope control under 44px. |
| T4 | **Input zoom.** On touch, every `input`, `select`, `textarea` and editable field is at least 16px, or iOS Safari zooms the page on focus. Do not use `maximum-scale=1`; it disables pinch zoom. | Focusing any field on the iPhone does not zoom the page. |
| T5 | **Touch action.** Range sliders get `touch-action: pan-y`. Drag surfaces get `touch-action: none`: `.ogm-box`, `.ogm-handle` and the rotate handle in `OgMakerModal.vue`. | On iPad, dragging an OG Maker box moves it and the page does not scroll. |
| T6 | **Flyout height.** `toggle()` in `SideBug.vue` sets `top` but the CSS max-height is `100vh − 24px`, so tall flyouts run off the bottom. Set max-height from the space below `top`. | At 844×390 and 1180×820 every flyout ends inside the viewport and scrolls internally. |
| T7 | **First-tap check (verify only).** iOS can treat a tap as hover when a hover handler changes the page. | On the iPhone, one tap activates the card title, a thumbnail and each side-bug button. |

## Phase 3 — Tuning entry on the phone

The target is one-handed entry of a full tune while reading the game's tuning menu: typed values first, sliders second, in the game's tab order, with nothing lost. All changes are in `TuningAdjustments.vue` unless noted.

| ID | Change | Done when |
| --- | --- | --- |
| U1 | **Two-line rows.** At ≤ 640px each `.ta-row` stacks: label and value box on line one; Low, full-width slider, High on line two. Override the `--col-*` variables rather than restyling each part. Today the track overlaps the Low value and High is cut off. | At 360 and 390px, `.ta-group` content fits its box and Low, slider and High are all visible. |
| U2 | **Numeric keypad.** `.ta-val-input` is `type="text"` with no hints, so the phone shows the full keyboard. Add `inputmode="decimal"` and `enterkeyhint="next"`, and select the value on focus. | Tapping a value shows the number pad with the old value selected. |
| U3 | **Next-field flow.** Enter/Next commits the value and focuses the next unlocked row's value box; after the last row of a tab, switch to the next tab and focus its first row. | A whole tab can be entered without touching a slider or closing the keyboard. |
| U4 | **Negative values.** The iOS number pad has no minus key. Rows whose minimum is below zero (camber, toe) get a ± button beside the value box on touch. | Camber −2.0 can be entered from the number pad. |
| U5 | **Step buttons.** On touch, − and + flank the value box and move one `r.step` per tap, repeating on hold. Reuse the ArrowLeft/ArrowRight path in `onRowKeydown` so clamp, undo and flush behave the same. | A value can be nudged by one step without the keyboard. |
| U6 | **Slider safety.** With `touch-action: pan-y` (T5), a vertical swipe that starts on a slider scrolls the page and leaves the value alone. On touch a tap on the track does not jump the value; only dragging the thumb changes it. Thumb at least 28px. | Ten scroll swipes across the sliders change no value. |
| U7 | **Undo on touch.** Undo is Cmd/Ctrl+Z only, and the snapshot is taken in `onSliderMouseDown`, which touch never fires. Capture on `pointerdown`, and add an Undo button to the panel header in edit mode. | A touch drag followed by Undo restores the previous value. |
| U8 | **Tab bar.** `.ta-tabbar` sticks to the top of the viewport while its panel is on screen, centres the active tab on change, hides its scrollbar and fades at the right edge when more tabs exist. Tabs 44px tall on touch. | With the keyboard open, the active tab name is still visible. |
| U9 | **Inline view.** "View Inline" (stacked mode) uses the same two-line rows. | No clipping in stacked mode at 360px. |
| U10 | **List of tweaks.** The non-stock summary columns are 38px wide and clip at phone width. One column at ≤ 640px. | Every tweak line reads in full at 360px. |
| U11 | **Dialogs.** Transmission picker, preset select and save-as-preset fit 360px with 16px fields. | Each opens, fits and closes at 360×640. |
| U12 | **Build parts** (`RecipeSection.vue`, `UpgradesPicker.vue`, `CarPicker.vue`): spec selects, upgrade steppers and the car search usable by touch. `CarPicker` selects on `mousedown`; confirm it selects on first tap. | A car, five specs and a full upgrade list can be set on the iPhone. |
| U13 | **Screen awake.** While a tuning panel is in edit mode on touch, hold a Screen Wake Lock and release it on exit or when the page is hidden (decision D4). | The phone does not dim during a five-minute entry session. |

## Phase 4 — Edit chrome and saving

On a phone, edit mode's controls fold into the side drawer and Save sits where the work happens.

| ID | Change | Done when |
| --- | --- | --- |
| E1 | **Edit controls in the drawer.** At ≤ 640px in edit mode, `.tl-dock` and `.edit-pill` in `EditBar.vue` are hidden and their actions (Exit, Jump to next edit, Save All with the unsaved count, New Card) appear in the opened side bug. The collapsed tab shows the unsaved count and pulses while edits are unsaved. Today the dock covers Build It and the section chevrons. | No card content sits under a fixed control other than the collapsed tab. |
| E2 | **Save where you work.** In edit mode the tuning panel's sticky header (U8) carries a Save button for that card (`ui.saveCard`), beside Undo. The result toast (`ToastStack.vue`) stays clear of the bottom edge. | A tune change is saved in one tap from inside the tuning panel, with a visible confirmation. |
| E3 | **Overlapping buttons.** `.edit-card-btn` in `CardMeta.vue` is absolutely positioned and lands on the collection chips at 390px; "History" does the same in the `EditCardModal.vue` header. At ≤ 640px put both in normal flow. | Neither button overlaps a chip at 360px. |
| E4 | **Draft safety** (decision D5). Unsaved edits live only in memory, so a phone that reloads the tab loses them. Write dirty cards to `localStorage` on change and offer to restore them on load. | Reloading mid-entry offers the unsaved tune back. |
| E5 | **Card text and New Card.** Title, subtitle factoid selects, share codes, tags, collections and the New Card modal are in scope for phone editing. Most of the gap should close with L1–L3, T3 and T4, so re-run the harness on these after Phase 2 and fix only what still fails. | A card can be created and its text edited on the iPhone at 360px with no clipping and no zoom. |

## Phase 5 — Photos, slideshow and lightbox

Building a card's slideshow from the phone needs a photo picker that works on phones and a way to reorder without drag-and-drop.

| ID | Change | Done when |
| --- | --- | --- |
| G1 | **Add photos.** Both upload inputs use `webkitdirectory` (`Gallery.vue`, `ImagePicker.vue`), which cannot pick from a phone's photo library. On touch render them as plain `multiple` inputs; `onFolderSelected` already handles files with no folder path. | Three photos chosen from the iPhone library land on the card. |
| G2 | **Rejected files.** HEIC is not decoded by the backend and skipped files are only logged to the console. Show the existing "export as JPEG" message as a toast. Check on device whether iOS converts to JPEG when `accept` lists only JPEG/PNG/WebP. | A rejected photo produces a visible message. |
| G3 | **Reorder without drag.** Thumbnails reorder by HTML5 drag, which is unreliable on touch. In the photo manager on touch, add move-earlier and move-later buttons that call `store.reorderImages`. Mouse drag stays. | Photo order can be changed on the iPhone. |
| G4 | **Photo manager.** At ≤ 640px `.image-picker-panel` is full screen, three columns, with lead, include and delete as 44px always-visible controls. | Lead image and slideshow membership can be set on the iPhone. |
| G5 | **Thumb rail.** A 96×60 thumbnail cannot hold three 44px controls. At ≤ 640px the rail only selects and the manager does the editing; this overrides T2 for the rail at phone width. | No control in the rail is under 44px at 390px. |
| G6 | **Lightbox.** Add left/right swipe (pointer events, horizontal move over 40px). At ≤ 640px move `.lightbox-ear` below the image; today the arrows sit on it. Close button 44px. | Swiping changes the image on the iPhone and nothing covers it. |
| G7 | **Naming photos.** `PhotoDetail.vue` holds each photo's alt text, car and livery. At ≤ 640px it fits the screen with 16px fields and the image above the fields. | A photo can be named and tagged to a car on the iPhone at 360px. |
| G8 | **Playback.** Tapping the stage still toggles play and pause; the expand button is visible without hover (T2). | Play, pause and open-in-lightbox each take one tap. |

## Phase 6 — Flyouts and desktop-only surfaces

At phone width the flyouts keep floating beside the opened side bug but always fit the screen, and the two designers are limited as agreed in Scope.

| ID | Change | Done when |
| --- | --- | --- |
| S1 | **Flyouts fit.** At ≤ 640px the filters, theme and text-size flyouts still open beside the side bug. Constrain each to the space available: width to the viewport minus the bug and 8px margins, height per T6, internal scroll, outside tap closes. No bottom sheets. | Each flyout opens, scrolls and closes at 360×640 with nothing off screen. |
| S2 | **Filters content.** `.filter-color-grid` and `.filter-tune-grid` are fixed at 154px wide. Let them take the flyout's width, with 44px rows and chips on touch. | Every filter can be toggled with a thumb. |
| S3 | **Theme Builder is desktop-only.** Hide the "Customize" entry at ≤ 640px and on touch devices (`hover: none`). Theme switching stays everywhere. | Not reachable at 390px or on iPad; reachable at 1440px with a mouse. |
| S4 | **OG Maker**. Hide the share-card design block in `ShareModal.vue` at ≤ 640px. It stays on iPad, where T5 makes its drag handles work. | Not reachable at 390px; usable at 820px. |
| S5 | **Modal gutters.** At ≤ 640px `.nc-overlay` padding drops from 40/20px to 12/8px so modals use the width. | New Card and Edit Card use at least 344px at 360px. |

## iPad

No iPad-specific rules are needed: the iPad takes the desktop layout from the width axis and the touch fixes from the capability axis.

| Situation | Layout | Touch rules | Extra work |
| --- | --- | --- | --- |
| iPad portrait, roughly 744–1032px wide | Desktop | Yes | L6: the side bug overlaps the card below 1067px, so most portrait iPads need the reserved gutter |
| iPad landscape, roughly 1024–1376px wide | Desktop | Yes | L6 on the narrowest models only |
| iPad in Split View or Slide Over, down to about 320px | Phone | Yes | None; it is a phone-width viewport |
| iPad with a trackpad or mouse | Desktop | Expected yes, since touch should still report as the primary input; confirm on device | None. Larger targets and no tooltips are harmless there |
| Narrow desktop browser window | Phone at ≤ 640px | No | None |

The 768×1024 run of this audit showed no clipping or overflow, which is why the tablet list is short. What iPad gains from this work order is Phase 2 in full, plus T5 for the OG Maker if it stays available there. Theme Builder is desktop-only.

## Test matrix and done criteria

The work is finished when the harness is clean at six viewports and two real-device runs pass on the iPhone.

**Harness (every phase)**

- Desktop parity: the 1440×900 set matches `docs/mobile-pass/before/`, apart from changes approved in a phase plan.
- Overflow audit clean at 360×640, 390×844 and 820×1180.
- Touch audit clean for in-scope controls at all touch viewports.
- Flyouts inside the viewport at 844×390 and 360×640.

**iPhone run 1 — tune entry**

- [ ] Sign in at `/#ignition` and open a card's tuning panel in edit mode.
- [ ] Enter every tab of a real tune from the game using the number pad and Next, including one negative camber value.
- [ ] Nudge two values with the step buttons and undo one slider drag.
- [ ] Scroll the page across the sliders; no value changes.
- [ ] Save, reload, and confirm every value persisted.

**iPhone run 2 — slideshow**

- [ ] Add three photos from the library to a card.
- [ ] Set the lead image, exclude one photo, move one earlier.
- [ ] Save, then swipe through the result in the lightbox.

**Docs to update at the end**

- `docs/shakedown.md` section 17: replace the ten generic checks with the two runs above.
- `docs/plan.md` item 2: mark done and link the after-screenshots.
- `.claude/skills/frontend-gotchas/`: add the hover-versus-touch and iOS input-zoom pitfalls.

## Decisions

All seven design decisions and every Scope row are answered as of 2026-10-09; nothing is waiting on Jason.

| ID | Question | Decision | Status |
| --- | --- | --- | --- |
| D1 | What does the side bug become on a phone? | A collapsible side drawer, not a bottom bar: it collapses to a small tab on the right edge and, when opened, floats over the side as it does now. Nothing is pinned to the bottom edge, where the iOS toolbar lives. | Settled |
| D2 | Should the Low and High range fields be editable on the phone? | Yes, with the number pad. | Settled |
| D3 | On touch, should tapping the slider track jump the value there? | No. A browser's stock slider jumps the thumb to wherever the track is tapped; desktop already blocks that on a row that is not focused, and touch will match. Dragging the thumb is the deliberate action. | Settled |
| D4 | Keep the screen awake while a tuning panel is in edit mode? | Yes. | Settled |
| D5 | Keep unsaved edits across a reload (draft safety, E4)? | Yes, in Phase 4. | Settled |
| D6 | Add a full-screen "tune entry" mode that shows only the tuning panel? | Not in this pass. Judge on the phone once the inline panel works. | Deferred |
| D7 | Add shake to undo for errant slider moves? | Not in this pass. Typed values already get iOS's own shake to undo. For sliders it needs a motion-permission prompt on iOS and can collide with that system dialog, so ship the Undo button (U7) first and add shake only if the button proves too slow in use. | Deferred |
