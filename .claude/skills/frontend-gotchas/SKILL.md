---
name: frontend-gotchas
description: Known Vue/TypeScript/CSS pitfalls specific to this codebase (vue-tsc quirks, focus-handling traps, reactive-graph divergence, overflow CSS spec behavior, pointer-events side effects). Load before debugging unexpected frontend behavior or writing new interactive components.
---

# Frontend Gotchas — Livery Catalog

## CSS `overflow-x: auto` implies `overflow-y: auto`

Setting `overflow-x: auto` on an element implicitly promotes `overflow-y` from `visible` to `auto` (CSS spec: the two overflow axes cannot have one `auto` and the other `visible`). This makes the element a vertical scroll container unexpectedly, causing bounce/swipe behavior on trackpads. Always pair with an explicit `overflow-y: hidden` when horizontal-only scroll is intended.

## `pointer-events: none` also kills `:hover`

Using `pointer-events: none` to make a control non-interactive also silently removes all hover feedback — the element becomes invisible to the mouse entirely. Use `opacity` + `cursor: default` when the element should still look interactive but do nothing, or accept that hover is gone when you use `pointer-events: none`.

## `vue-tsc` gotcha

String template refs (`ref="x"`) aren't counted as "used" by `vue-tsc`'s unused-locals check. When a composable needs an element ref, create it in the component and **pass it into the composable** (so it's read in script) — see `Gallery.vue` passing `stageRef`/`barRef`/`toggleRef` into `useSlideshow`.

## Imports must be at the top of `<script setup>`

An import placed after `defineProps`/`defineEmits` silently breaks Vite HMR for that file — code changes on disk have no visible effect even after a hard refresh, because the server is handing out a stale transform. The fix: move the import to the top, then restart the dev server (`npm run dev`).

## `e.preventDefault()` on `mousedown` blocks focus

Calling `e.preventDefault()` on a `mousedown` event blocks the element from receiving keyboard focus — not just the default interaction you intended to suppress. Whenever you use this pattern (suppressing a range input's jump-to-position, blocking a drag-start, etc.), manually call `.focus({ preventScroll: true })` on the element that should own keyboard events next. See `onSliderMouseDown` in `TuningAdjustments.vue` for the reference implementation.

## `focusedKey` ≠ DOM focus (`TuningAdjustments`)

In `TuningAdjustments.vue`, `focusedKey` is a reactive ref that drives the visual highlight ring. It has nothing to do with `document.activeElement`. Setting `focusedKey` without also ensuring a DOM element inside that row has focus produces a highlighted row that ignores all keyboard events — arrow keys fall through to the browser's scroll behavior.

## Props are a separate reactive graph from the Pinia store

When a parent passes a deep-cloned reactive object as props (e.g. `RecipeSection` receiving a `local` recipe copy), **never bypass those props to read the same data from a store**. The clone and the store diverge immediately on first edit — the store still holds the pre-edit value. Read from props; flush to store explicitly. This is why `TuningAdjustments` reads `upgrades` and `coreSpecs` from props, not from `cards.byId()`.

## Multi-column layout decision framework

- **CSS grid** — when items in different columns share a common baseline or you want aligned rows across columns.
- **CSS `columns`** — when each column should be independently tall. Use `break-inside: avoid` to keep blocks intact; `break-before: column` to force a new column at a specific block. Works only when all blocks are similar in height — `column-fill: balance` will still break a block that exceeds the target column height.
- **Explicit column divs + JS height balancing** — when blocks vary wildly in height (one may be 3× taller than another) or you need a guaranteed no-break. Assign blocks to the shortest column with a greedy algorithm; each column is an independent `flex-direction: column` container. See `tweakColumns` computed in `TuningAdjustments.vue` and `.up-picker` in `UpgradesPicker.vue` for both patterns.

## Virtual scroll slot recycling

`vue-virtual-scroller` is no longer used in this codebase — it was removed due to two failure modes (slot recycling resetting component state, and duplicate concurrent pool slots). See `frontend-patterns → Card list rendering` for the full history and the correct alternative.

## `ui.ts` ↔ `modal.ts` circular imports are SETTLED — do not "fix"

The two stores are mutually dependent (401 → open login; login success → enter edit) and use `import('./modal')` / `import('./ui')` to break the module-level cycle. The build's `INEFFECTIVE_DYNAMIC_IMPORT` warning is **expected and benign** — it complains the import won't get its own chunk, but chunking was never the goal. Reviewed twice (audit pass 4, review 2026-07-17): a signal-module extraction only fixes one direction and would force post-login flow into `LoginModal.vue`, scattering logic the stores deliberately centralize. Full rationale in `ui.ts → handleAuthError`. (`theme.ts` was never part of the cycle and imports `ui` statically.)

## Touch: interaction follows capability, not width

Phone/iPad behaviour hangs off `@media (hover: none), (pointer: coarse)` (CSS) or `isTouch` (`composables/isTouch.ts`), never width or user agent — iPadOS Safari reports itself as a Mac, and an iPad is desktop-width. Layout follows width (`max-width: 640px`); overrides live in `styles/responsive.css` or the component's scoped style. A tap fires `mouseenter`, so anything hover-driven sticks open on touch: `v-tip` only attaches its listeners when `matchMedia('(hover: hover)')` matches.

## iOS zooms into any field under 16px

Focusing an `input`/`select`/`textarea`/contenteditable with a computed font-size below 16px makes iOS Safari zoom the page. Raise the field to 16px on touch (`font-size: max(16px, 1em)` under `:where(...)` so class rules still win, or 16px in the component's touch block). Never use `maximum-scale=1` — it disables pinch zoom.

## Buttons that act on `pointerdown`: guard against tap-through

The tuning steppers act on `pointerdown` (for hold-to-repeat). The tap still produces a click afterwards, and if the step changed what's under the finger, the click lands there: it moved focus off the value box being typed in, and when a step opened the Springs prompt it pressed "Reset Stock" (zeroing a tune). `@touchstart.prevent` on the button stops most of it but is not reliable in Chromium. So also guard what can appear: a prompt that a step can open ignores presses for 400ms after it opens (`promptReady()` in `TuningAdjustments.vue`).

## `overflow: hidden` on an ancestor kills `position: sticky`

`hidden` makes the ancestor a scroll container, so a sticky child sticks to that box (which never scrolls) instead of the viewport. `.card` uses `overflow: clip` on phone/touch for the tuning panel's sticky tab bar — same clipping, no scroll container.

## An invisible `::after` hit area needs a positioned host

44px touch hit areas are an absolutely positioned `::after` on the control. If the control is later made `position: static` (e.g. moved into normal flow on phones), the `::after` anchors to the nearest positioned ancestor instead — it ended up over the middle of the card, intercepting taps on the tuning panel. Use `position: relative` to put such a control back in flow.

## Playwright: tall element screenshots drop touch emulation

`locator.screenshot()` on an element taller than the viewport (beyond-viewport capture) silently resets touch emulation: `(hover: none)` / `(pointer: coarse)` stop matching for the shot and for everything after it. `frontend/shot.mjs` grows the viewport to fit the element instead, and asserts touch media before every touch audit.
