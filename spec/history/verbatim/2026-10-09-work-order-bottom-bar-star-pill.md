# The Livery: bottom bar, star and Playing pill — work order for CC

From App Claude, 2026-10-09, after Jason's first look at the mobile pass
on his phone. Three parts, in his priority order. It extends the Mobile &
Touch work order and keeps its ground rules: two axes (layout follows
width at `max-width: 640px`, interaction follows `(hover: none),
(pointer: coarse)`), `catalog.css` untouched, desktop at 1440×900
pixel-identical in the harness, one part per commit, plan mode first,
commit and push only when Jason says.

Do this before the Projects-standard work order in this folder: this
one names `docs/` paths and that one renames the folder.

**Superseded in the Mobile & Touch work order:** decision D1, the phone
clause of L6 (the collapsed edge tab), and E1 (edit controls in the side
drawer). L6's 641–1066px gutter stays as built.

Findings are labelled Confirmed (read in the code, as named), Precaution
or Conjecture. Nothing here was checked on a device.

## Part A — the side bug becomes a bottom bar on phones

Jason, 2026-10-09: the side tab is "more distracting and harder to use";
"let's try the bottom, and hide it rather than move it for keyboard
engagement."

| ID | Change | Done when |
|---|---|---|
| A1 | **A floating pill at the bottom.** At ≤ 640px `.side-bug` is a centred row: `position: fixed`, `bottom: calc(10px + env(safe-area-inset-bottom))`, in the language of desktop's `.edit-pill` (`EditBar.vue`: panel mix, accent border, blur, full radius). Remove the collapsed state: `collapsed`, `expandBug()`, `.bug-tab`, `.bug-tab-count`, the pulse. `positionSideBug()` sets inline `top` and `right`; at phone width it must set neither and clear what it set. 641px and wider is unchanged. | At 360 and 390px the bar is inside the viewport, centred, every button 44×44. 1440 parity identical. |
| A2 | **Two sets, same DOM order.** Viewing: Account and Edit (signed in), Filters, Theme, Text size, Expand/collapse: six at most, 264px. Editing: Edit (shown active, exits), Save All, Next edit, New Card, Filters, Expand/collapse; Account, Theme and Text size are hidden while editing. No "more" menu. | Six buttons or fewer in both sets at 360px, signed in. |
| A3 | **Unsaved count on Save.** Save All carries the count (`.bug-sugg-dot`, as now) and is drawn filled (`.bug-btn.active`) while anything is unsaved. No pulsing. | With two dirty cards the bar shows Save filled with "2". |
| A4 | **Flyouts open upward.** `toggle()` places a flyout by `top` and `right` beside the bug. At phone width place it by `bottom` (the bar's top edge plus 8px), 8px side margins, `max-height` from the space above the bar, measured from the bar's own rect so it is right whether Safari's toolbar is showing or not. Internal scroll, outside tap closes. Still a popover, not a sheet. | Filters, theme and text size open, fit, scroll and close at 360×640 and 390×844; none overlaps the bar. |
| A5 | **Hide while typing; never move.** While focus is in a text field (`input` other than range, checkbox, radio, file; `textarea`; `[contenteditable=true]`) the bar and any open flyout are hidden (opacity and `pointer-events`, about 150 ms). They return about 250 ms after focus leaves, unless it went to another field: the Next-field flow and `.ta-focus-proxy` hop between fields and must not flash the bar. No `visualViewport` arithmetic and no repositioning above the keyboard. | In the harness: focusing a tuning value hides the bar; Enter through three rows never shows it; blur brings it back. |
| A6 | **Clearance.** Phone `body` bottom padding becomes `calc(96px + env(safe-area-inset-bottom))` so the last card clears the bar. The bar stays under every overlay (lightbox, photo manager, New Card, Edit Card, dialogs). | Page end: no card content under the bar. With each overlay open the bar cannot be seen or tapped. |
| A7 | **Harness.** `shot.mjs`: replace the phone side-bug checks with the bar's (inside viewport, 44px targets, clear of `.ta-header`), flyouts checked against the bar, and a focused-field view showing the bar hidden. | The audit reports all of it at 360×640 and 390×844. |
| A8 | **Docs.** `docs/shakedown.md` §17: replace the "Side drawer" row with the device checks below. `docs/plan.md`, the AAR, and the side-bug notes in both skills. | A grep for "edge tab", "side drawer" and "bug-tab" finds nothing stale in live docs. |

**Device checks for §17 (Jason's hands)**

- [ ] The bar sits just above Safari's toolbar, and follows it down when the toolbar collapses on scroll.
- [ ] Nothing of the bar is under the home indicator.
- [ ] Tapping a tuning value hides the bar; closing the keyboard brings it back in the right place.
- [ ] Each bar button works on the first tap.

Precaution: iOS 26.0 Safari was reported to misplace fixed bottom
elements after the keyboard closes, said to be fixed in 26.1. A5 hides
the bar while the keyboard is up, but the third check is the one that
would show it. If it fails, report it; do not chase it without a device.

## Part B — the favourite star

Jason: "Stars turn on but not off for favoriting a card." And of the
mouse: "the state change is supposed to show greyed and not retrigger
the hover until clear and return."

**The look.** Cause (Confirmed in the code): `.fav-star:hover` paints
the accent colour (`catalog.css` 311–314). Touch keeps `:hover` on the
last thing tapped, so an un-favourited star stays gold until something
else is tapped. The data does toggle (`cards.ts` `toggleFavorite`). The
original's cure for the mouse, `.fav-star.suppress-hover` (`catalog.css`
316–322), was never wired up in the Vue port: nothing in `src/` sets
that class. Jason has seen that on desktop too.

**Where a star is kept.** Jason: "The star is a favorites toggle and
should be persistent through the ad hoc user cookie whatever thing we
did to let users return with certain memory of what they've interacted
with... It's part of the sort system." **That memory does not exist in
the code** (Confirmed by grep, 2026-10-09): there is no cookie and no
visitor id anywhere, front or back. `localStorage` holds only the
sign-in, drafts, the colour palette, recipe presets and two admin
lists. `ui.ts` starts every visit at `theme = 'dark'` and `textDelta =
0` and never stores either, although `frontend-patterns` says theme and
text size are kept in `localStorage`. Today `toggleFav` in
`CardMeta.vue` flips `isFavorite` on the card and calls
`ui.markCardDirty`: outside edit mode nothing can save that, a reload
loses it, and the dirty card arms "Leave site?" (`ui.ts` 103–106), the
prompt the phantom-card fix just removed.

**What the visitor memory is, and is not.** It holds what the docs
already say is kept per browser and is not: stars, the theme pick and
text size. The keys that do work today stay where they are (recipe
presets `tl-recipe-presets`, the colour palette `cp-palette`, drafts,
the sign-in). The "cookie" Jason remembers is the **guest theme
adjuster** in `docs/completed/plan-theme-builder.md` ("Future — Guest
theme adjuster"): planned, not built, and specified as a cookie so the
server can read a guest's choices. That feature is not part of this
order. Keep the module's reading and writing behind two functions so
its backing can become that cookie when the adjuster is built.

| ID | Change | Done when |
|---|---|---|
| B1 | **Touch look.** Under the capability query in `responsive.css`: `.fav-star:hover:not(.favorited)` is star-grey with no scale. | At 390 touch, two taps on a star: its computed colour is grey again with no other tap. |
| B2 | **Mouse look: hover is an invitation over the state** (the table below). Wire `suppress-hover` in `CardMeta.vue`: set by any click on the star, cleared on `mouseleave`. Add the brighter hover for a starred star (`--accent-bright`, already in `catalog.css`), in `CardMeta.vue`'s scoped style or `responsive.css` under `(hover: hover)`; no edit to `catalog.css`. Mind one trap: `catalog.css`'s `.fav-star.suppress-hover:hover` is grey and outranks `.fav-star.favorited`, so a star just switched on needs its own rule to show plain yellow while suppressed. Check whether `.fav-star.pop` is wired while there. **This changes what a mouse sees, by Jason's request**; the 1440 parity shots have no hover in them and must stay identical. | Each row of the table below, checked at 1440 with a mouse by computed colour. |
| B3 | **A visitor's memory.** One small module (say `composables/visitorMemory.ts`): a single versioned `localStorage` key, read once at start, written on change, every access in try/catch so a blocked or full store costs nothing but the memory. No cookie, nothing sent to the server. | Unit of its own; the app runs unchanged with storage blocked. |
| B4 | **Stars live there.** A star is the visitor's own: the set of favourited card ids is kept in the visitor memory. The star, and "Favorites only" in `filters.ts`, read that set. `toggleFav` stops calling `markCardDirty` and stops writing the card. The card's stored `isFavorite` is no longer read or written; leave the field in the type and the data (no migration). | Signed out at 390 and 1440: star two cards, reload, both are still starred and "Favorites only" shows exactly those two. `ui.dirtyIds` stays empty and no "Leave site?" appears. |
| B4a | **Theme and text size live there too.** Three docs say both are kept per browser (`frontend-patterns`, `docs/completed/DESIGN_SYSTEM.md`, `docs/completed/THEME_PANEL_SPEC.md`); no commit ever stored either (Confirmed: `git log -G localStorage` on `ui.ts` is empty). Store the visitor's ambiance pick and `textDelta`, and apply them at start. Mind the order with `themeStore.load()`: `pickTheme` goes through `themeStore.setAmbiance` when a site theme is loaded, so the visitor's pick is applied after that load, the same way. | Pick Light and +2, reload: both are back, signed in or out. A first visit still opens on the site's default. |
| B5 | **Sweep (report only).** List every other `:hover` rule that paints a control's "on" look. Fix the ones on in-scope controls the same way; the rest go to `docs/plan.md`. | The list is in the phase report. |
| B6 | **Docs, and a sweep.** Grep the live docs and both skills for every claim that something is remembered per browser, and check each against the code. Fix the docs to say what is true after this part (`frontend-patterns`' persistence table; `docs/architecture.md`'s `isFavorite` line). Report any claim that is still not built. | No live doc claims a persistence the code lacks; the report lists what was checked. |

**The star with a mouse** (Jason, 2026-10-09: "We had to be specific
because deactivating the star was leaving the bright hover and no
visual distinction occurred that state had changed.")

| The star is | The cursor | It shows | Why |
|---|---|---|---|
| Off | Away | Grey | The state |
| Off | Over it | Yellow | It can be pressed, and this is what a press gives |
| On | Away | Yellow | The state |
| On | Over it | A little brighter than on | It can be pressed; brighter, so hover and on are never the same look |
| Just clicked, either way | Still over it | The plain result: grey if now off, plain yellow if now on | The click's result must be what shows. A second click at once toggles again and again shows the result |
| Any | Left, then back | Hover dressing as in rows 2 and 4 | The invitation returns only after the cursor has gone and come back |

**On touch there is no invitation layer:** the star shows its state and
nothing else (B1).

## Part C — the Playing pill

Jason: "The overlay of Playing is persistent in the card slideshow, but
disappears after a time on the web version... In mobile, scrolling a
touch should wake it for a brief moment. When paused it can persist."
And, corrected the same evening: "on the browser the behavior was
already correct... mousing into the frame wakes the overlay to show the
user that an action can be taken. I suppose that if the mouse is still
there for whatever the time is for idle, then it too can disappear like
netflix does."

So: **the mouse behaviour is the reference and stays.** The phone is
brought to the same idea in touch's own terms, and the mouse gains one
thing, an idle fade.

Causes on the phone. Confirmed in the code: `toggle()` in
`useSlideshow.ts` calls `setSettled(false)` and nothing sets it back,
so after one manual resume "Playing" stays up. On a phone the stage
itself is the play/pause control, so the first tap gets there. The same
path exists for a mouse (pause, then play, by the pill), though Jason
has not seen it there: reproduce it at 1440 first and say what you
find. Likely also (Conjecture): `.stage.settled:hover .play-toggle`
holds the pill up while touch `:hover` sticks to the stage.

The rule: **while playing the pill sleeps. A mouse over the stage, or a
touch, wakes it. While paused it stays.**

| ID | Change | Done when |
|---|---|---|
| C1 | **Sleep after every start.** Any route into playing (the first reveal, the pill, a tap on the stage) ends with the pill asleep (`.settled`). The first reveal keeps its 2 s + 2 s choreography. A resume shows "Playing" briefly, then sleeps. | Pause then play, cursor or finger away: the pill is gone within about 2 s, at 1440 mouse and 390 touch. |
| C2 | **Mouse: as it is, plus idle.** Mousing into the stage still wakes the pill and it still stays while the mouse is in use there. New: after about 3 s with no pointer movement over the stage, it sleeps; the next movement wakes it. Add a class for the idle state and a rule beside the existing ones; no edit to `catalog.css`. Mouse-only: under `(hover: hover)`. | At 1440: cursor resting on a playing stage for 5 s, no pill. Move it: the pill is back. Move out and in: as today. Parity screenshots identical. |
| C3 | **Touch: a wake, not a hover.** Under the capability query the `:hover` hold is switched off, so a stuck touch hover cannot keep the pill up. A wake shows the pill for about 1.5 s, then the normal fade; one timer per stage, restarted by a new wake. Wakes: a touch that starts on the stage, and a page scroll while a playing stage is on screen (passive listener, throttled, intersecting stages only). Constants beside `BUTTON_REVEAL`. | At 390 touch: scrolling with a playing card in view shows the pill, and it is gone about 2 s after the scroll stops. |
| C4 | **Paused persists.** Paused by the pill, the stage or a thumbnail: the pill stays, reading "Paused", on both. | Still there at 10 s. |

## Decisions

| ID | Question | Decision | Status |
|---|---|---|---|
| D1 | Side bug on a phone | A bottom bar, replacing the edge tab (reverses the Mobile & Touch work order's D1) | Settled |
| D2 | Keyboard up | Hide the bar; do not move it | Settled |
| D3 | Edit set | Six buttons, with Account, Theme and Text size hidden while editing, in place of a "more" menu. App Claude's proposal; Jason judges on the phone | Settled, to judge on device |
| D4 | Pill timings | Touch wake about 1.5 s; mouse idle about 3 s. Jason judges in use | Settled, to judge in use |
| D5 | What is a star? | The visitor's own favourite, kept in their browser and part of the sort system (Jason, 2026-10-09). Built in B3 and B4 | Settled |
| D6 | Stars already saved on cards | They are leftovers from test clicks and their state does not matter (Jason, 2026-10-09). B4 as written: the stored `isFavorite` is ignored from now on, no migration | Settled |
| D7 | What else does a visitor's memory hold? | Everything specified as kept per browser that was never built: today that is the theme pick and text size, with the stars (Jason, 2026-10-09). Built in B4a. If the sweep of the docs in B6 finds another, list it and ask | Settled |
