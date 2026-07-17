# Handoff: Theme Builder — Picker Wing Glass Surface

**Date:** 2026-07-03  
**Status:** In progress — transparency approach identified and partially working

---

## What the feature is

The Theme Builder flyout has three side-by-side surfaces:

```
[ picker wing (272px) ][ tab (14px) ][ list panel (300px) ]
```

- **Right panel** (`.tb-panel`) — the scrollable list of editable colors/settings. Correct opaque-ish frosted glass.
- **Center tab** (`.tb-picker-tab`) — a narrow toggle strip, always visible, that opens/closes the picker wing.
- **Left picker wing** (`.tb-picker-wing`) — slides out when a color is selected, contains the `ColorPicker` component.

**Design intent:** The picker wing and the center tab should read as **one unified lighter glass surface** — clearly more see-through than the right panel. The right panel is the "base" (darker, more solid); the wing+tab together float in front of it as a lighter overlay.

---

## What's currently in the code

### The working part

The right panel (`tb-panel`) uses `background: var(--glass-bg)` from its scoped CSS. `--glass-bg` is defined in `catalog.css`:

```css
--glass-bg: color-mix(in srgb, var(--panel) var(--glass-opacity), transparent);
```

`--glass-opacity` is set on `document.documentElement` by the theme store's `applyEffects()` (defaults to `82%`). The opacity slider in the Effects section changes this in real time. This all works correctly.

### The broken part — what was tried and why it failed

The goal was to give the wing and tab a lighter version of the same glass. Five approaches all failed:

1. **`var(--picker-glass-bg)` in scoped CSS** — defined `--picker-glass-bg: color-mix(in srgb, var(--panel) 28%, transparent)` in `catalog.css`. No visual change. Root cause: all CSS custom property vars defined in `catalog.css :root` that contain `color-mix()` return an empty string from `getComputedStyle().getPropertyValue()` in Safari — they exist and render correctly for elements that inherit them, but the chain breaks in specific contexts.

2. **`color-mix()` directly in scoped CSS** — `background: color-mix(in srgb, var(--panel) 15%, transparent)` — no change. Same Safari issue: `color-mix()` with `var()` inside does not resolve when used directly in a property value (only works when stored as a CSS custom property first, then referenced via `var()`).

3. **`color-mix()` via Vue `:style` binding** — `background: 'color-mix(in srgb, var(--panel) 15%, transparent)'` — same result. Verified in browser: resolved to `rgba(0,0,0,0)`.

4. **Root cause found — nested backdrop-filter** — The outer `.bug-flyout--builder` container inherits `backdrop-filter: blur(16px)` from the base `.bug-flyout` class in `catalog.css`. When a parent has `backdrop-filter`, it creates a new CSS stacking context. Children with their own `backdrop-filter` then blur WITHIN that stacking context (blurring the already-blurred parent output), not the raw page. This made all child surfaces look visually identical regardless of their `background` value. Fixed in `SideBug.vue` scoped CSS:
   ```css
   .bug-flyout--builder {
     backdrop-filter: none !important;
     -webkit-backdrop-filter: none !important;
   }
   ```

5. **Local `--glass-opacity` override via `:style`** — Set `--glass-opacity: '15%'` as a Vue inline style on the wing and tab elements, keeping `background: var(--glass-bg)` in scoped CSS. Did not work — the local custom property was not picked up when resolving `--glass-bg`.

### Current approach (partially working, needs visual tuning)

The fix in place: **compute the background in JavaScript** from `theme.current?.colors.panel` and pass a plain `rgba()` value as a Vue `:style` binding. `rgba()` has no variable resolution chain, works in all browsers.

**`ThemeBuilder.vue` script:**
```typescript
const pickerBg = computed(() => {
  const hex = (theme.current?.colors.panel ?? '#15151a').replace('#', '')
  const n = parseInt(hex.length === 3 ? hex.split('').map(c => c+c).join('') : hex, 16)
  return `rgba(${(n>>16)&255},${(n>>8)&255},${n&255},0.18)`
})
```

**`ThemeBuilder.vue` template:**
```html
<div class="tb-picker-wing" :style="{ background: pickerBg }" ...>
<button class="tb-picker-tab" :style="{ background: pickerBg }" ...>
```

Currently at **0.18 opacity**. This was the value at context handoff — it hasn't been visually confirmed as correct yet.

---

## What needs to happen next

1. **Dial in the opacity value.** The `0.18` in `pickerBg` is a starting estimate. Open the theme builder, expand the picker wing, and compare the left surface to the right panel. The wing+tab should be noticeably more transparent/lighter. Adjust the `0.18` in the computed function up or down until it looks right. Reference screenshot: `docs/Screenshot 2026-07-03 at 10.53.01 PM.png` (orange carets show desired gap transparency).

2. **Verify all 5 themes.** Switch through dark/light/rainbow/clouds/stormy and make sure the wing+tab look appropriately lighter than the panel in each. The `rgba()` value is computed from the current theme's panel color, so it should adapt automatically — but light themes (where `--panel` is near-white) may need a different alpha than 0.18.

3. **Consider extracting the alpha as a constant** if you want it easy to tune:
   ```typescript
   const PICKER_GLASS_ALPHA = 0.18
   const pickerBg = computed(() => { ... return `rgba(..., ${PICKER_GLASS_ALPHA})` })
   ```

---

## Files changed in this session

| File | What changed |
|------|-------------|
| `frontend/src/components/ThemeBuilder.vue` | `pickerBg` computed; `:style` binding on wing + tab; scoped CSS uses `var(--glass-bg)` for both |
| `frontend/src/components/SideBug.vue` | `backdrop-filter: none !important` on `.bug-flyout--builder` |
| `frontend/src/styles/catalog.css` | `--picker-glass-bg` added to `:root` (unused now but harmless) |
| `frontend/src/stores/theme.ts` | `glassOpacity` effects field; `panelWell` + `steelLight` color keys; `normalize()`; `setGlassOpacity()` action |
| `backend/src/main.rs` | `THEME_DEFAULT` updated with `panelWell`, `steelLight`, `effects.glassOpacity` |

---

## Known Safari gotchas (for future reference)

- `getComputedStyle(el).getPropertyValue('--some-var')` returns `""` for any CSS custom property whose value contains `color-mix()` or `var()` — even if the property is correctly defined and rendering correctly. Don't use this API to diagnose whether a custom property is working.
- `color-mix(in srgb, var(--something) X%, transparent)` used **directly** as a property value does not resolve when `--something` is set via `element.style.setProperty()`. It only works when the `color-mix()` expression is stored inside a CSS custom property definition (e.g., `--glass-bg: color-mix(...)`) and then referenced with `var(--glass-bg)`.
- Nested `backdrop-filter` (parent + child both have it) kills the child's ability to show distinct background colors — all children in the stacking context look the same shade of blurry grey.
