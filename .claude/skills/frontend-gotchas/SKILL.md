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
