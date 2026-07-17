# Plan: RecipeSection one-way data flow refactor

## The problem

RecipeSection holds a local reactive copy of the recipe (`local`) and has to stay
in sync with two simultaneous sources of change:

- **Outbound (user edits):** user changes something → `flush()` → emit `update:recipe`
  → parent writes to store → `props.recipe` updates.
- **Inbound (external reset):** parent replaces card data (history restore, discard/cancel)
  → `props.recipe` changes → component needs to reflect it.

These two paths form a loop: our own emit causes a prop update, which the watcher
picks up and tries to write back in, which triggers another emit, and so on. The
current code breaks the loop with two boolean flags (`skipNextPropsSync`,
`inPropsSync`) set manually before and during each direction of travel.

The flags work but are fragile: any new watcher that touches `local` without
knowing about `inPropsSync` silently reintroduces the cycle.

---

## The fix: separate "your own round-trip" from "genuine external reset"

Instead of watching `props.recipe` deeply and trying to ignore our own echoes, we
give the parent an explicit way to say "this is a real external reset, not a
round-trip from your flush."

**New prop: `resetToken`** — a `number` the parent increments whenever it wants
RecipeSection to re-read from `props.recipe` from scratch. RecipeSection watches
`resetToken` (not `props.recipe`) for inbound syncs. Since `resetToken` only
changes when the parent deliberately bumps it, there is no cycle:

- User edits → `flush()` → emit → parent updates store → `props.recipe` changes
  → `resetToken` is NOT bumped → RecipeSection ignores it ✓
- Parent does a history restore / cancel → parent bumps `resetToken` → RecipeSection
  re-reads `props.recipe` → no emit triggered → no loop ✓

The flags disappear entirely.

---

## New contract

```ts
// RecipeSection props (addition only — update:recipe emit is unchanged)
resetToken?: number   // parent increments to force a full re-read from props.recipe
```

RecipeSection internals:
```ts
watch(() => props.resetToken, () => {
  Object.assign(local, cloneRecipe(props.recipe))
})
```

That's the entire inbound sync path. No flags, no deep watch on `props.recipe`.

---

## Files that change

### `RecipeSection.vue`
- Add `resetToken?: number` to `defineProps`
- Replace `watch(() => props.recipe, ...)` with `watch(() => props.resetToken, ...)`
- Remove `skipNextPropsSync` and `inPropsSync` flags entirely
- Remove the flag-setting lines in `flush()`

### `CardView.vue`
- Add a `recipeResetToken` ref (starts at 0)
- Bump it when an external reset happens — the only current case is history restore
  via `restoreCardVersion()` in the store. CardView doesn't directly trigger this;
  it happens via the store, so we need to detect it. Options:
  - Watch the section's `tuneName` for unexpected changes (hacky)
  - Expose a `resetRecipe()` method on CardView that CardHistoryModal calls (clean)
  - Pass the token down and let the store bump it (overreach)
  - **Simplest:** watch `props.card` with `{ deep: false }` — a history restore
    replaces the whole card object reference via `cards.value.map(...)` in
    `restoreSnapshot()`, so a shallow watch fires. Bump the token there.
- Pass `:reset-token="recipeResetToken"` to RecipeSection

### `EditCardModal.vue`
- Add a `recipeResetToken` ref
- Bump it in `onCancel()` after restoring the recipe snapshot (currently mutates
  `recipe.*` in place — the token tells RecipeSection to re-read)
- Pass `:reset-token="recipeResetToken"` to RecipeSection

### `NewCardModal.vue`
- No change needed. It already resets by reassigning `recipe.value = blankRecipe()`
  which replaces the prop reference entirely, which Vue picks up naturally even
  without a deep watch. (The comment on line 80 confirms this was intentional.)

---

## What stays the same

- `emit('update:recipe', clone)` — unchanged, still the outbound path
- `@update:recipe` handlers in all callers — unchanged
- All template markup in RecipeSection — unchanged
- TuningAdjustments integration (`taRef`, `getAdjustments()`) — unchanged

---

## Risk assessment

**Low.** The outbound path (user edits → flush → emit) is not touched at all.
The inbound path (external resets) is used only in two real scenarios:
1. History restore in CardView
2. Cancel in EditCardModal

Both are easy to test manually. NewCardModal is unaffected. The flag removal
simplifies the code rather than adding complexity.

---

## What to learn from this for future builds

> **When a component needs to both emit edits and accept external resets, separate
> the two signals explicitly.** Give the parent a dedicated "reset" mechanism
> (a token, a method, a key) rather than watching the same prop that your own
> emits update. One-way data flow breaks down not because of bad intent but because
> "this prop changed" conflates two different meanings.

The key-based remount (`v-if` / `:key` swap) is the most common framework-idiomatic
solution but costs component state. The reset token is a lightweight alternative
when you need to preserve state across most updates and only reset on deliberate
parent action.
