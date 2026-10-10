# Tuning Facsimile — Spec

Status: starting doc, written against the working widget prototype (9 tabs,
filled slider track, stock-diff highlighting, auto-generated changes list,
tabbed/stacked toggle). Covers the three gaps identified after the prototype
was already interactive: per-card persistence of stock values, what "Define
Stock" should do on a second press, and where this component does and
doesn't belong in the app.

This replaces the shape of `recipe.adjustments[]` in `CLAUDE.md`'s `Card`
type — currently `{ name; description }[]`, which is what's being retired.

## 1. Data shape — stock is per-card, same as min/max

Already established for min/max in earlier passes: nothing about a car's
tuning range is a game-wide constant, it depends on the specific build. Stock
is the same kind of fact — a 1999 Viper's stock tire pressure isn't the same
number as a stock Miata's. So stock has to live on the card, not as a
hardcoded default in the component.

```ts
interface AdjustmentRow {
  tab: string        // 'tires' | 'alignment' | 'arb' | ... (not 'gearing' — deferred)
  group: string       // e.g. 'Tire Pressure', 'Camber', 'Center'
  key: string         // e.g. 'tiresFront', 'camberRear'
  label: string        // e.g. 'Front', 'Rear', 'Acceleration'
  unit: string
  min: number
  max: number
  stock: number
  value: number
  step: number
}

// recipe.adjustments: AdjustmentRow[]
```

**Only rows the card actually uses get stored.** The editor can show all 9
tabs with some greyed out (mirroring the in-game "locked until you install
X" reality), but a card with no aero kit simply has no `aero` rows in its
`adjustments[]` — there's no need to persist a `locked: true` placeholder for
something that was never tuned. This keeps cards that don't use a given
category from carrying empty/dummy data.

This is a plain field change on `Card.recipe.adjustments`, same category as
the subtitle work — threads through `types.ts` → the component, no backend
schema change.

## 2. "Define Stock" on a second press

Right now it silently overwrites every row's stock with its current value.
That's fine the first time (capturing the untouched baseline before tuning
starts) but dangerous afterward — by the time you've actually tuned the car
away from stock, a second accidental press destroys the exact comparison
data the highlighting and the auto-list exist to show. There's no undo.

**Decided approach — explicit engaged/disengaged dialog state, reusing the
existing `ExitConfirmModal` pattern rather than a new modal type:**

- **Disengaged (idle):** button reads "Set Stock Parameters." Normal editing.
- **Engaged:** clicking it opens a confirm dialog — "Proceed to update stock
  parameters with current values? This will overwrite stock for N changed
  value(s)." Two buttons:
  - **Cancel** (red) — closes the dialog, nothing changes, back to disengaged.
  - **Proceed** (green) — overwrites stock for every row, clears the diff
    highlights, closes the dialog, back to disengaged.
- **Exception:** if there are zero diffs when clicked (first capture, or
  everything already matches stock), skip the dialog entirely — nothing is
  at risk, no reason to force a warning click-through for a no-op.

**Open color-convention note, not resolved here:** red-for-cancel /
green-for-proceed is intuitive as a stop/go pairing, but it puts the *safe*
color on the irreversible, no-undo action and the *warning* color on the
button that protects you from it — backwards from the usual convention of
putting the warning color on the dangerous action itself. Both are
defensible; flagging so it's a deliberate choice, not an accident of
matching the sign-out button's red style.

This also covers the legitimate "I fat-fingered the original stock capture
and need to redo it" case — it's still one click plus a confirm, not locked
away behind anything heavier, just not silent.

## 3. Where this does and doesn't belong

**Edit (`EditCardModal` / `NewCardModal`):** the full interactive version —
draggable sliders, editable min/max, the Define Stock button, both tabbed
and stacked layout. This is the authoring tool: you're transcribing what's
on your screen in-game.

**Display (`CardView`'s existing Tune / Build Parts section):** a read-only
render of the same data and the same visual (filled track, thumb position,
stock tick, the auto-generated changes list) — but no dragging, no editable
min/max, no Define Stock button. The slider thumb position still needs to
render so visitors can see the value at a glance; it just isn't interactive.
This is a rendering-mode difference on the same data, not a second
component.

The tabbed/stacked toggle stays in *both* modes — that's a genuine display
preference for visitors too, not just an authoring convenience.

**Doesn't belong:**
- **The legend (`NO. 000`) card** — worth deciding explicitly whether it
  shows a fully populated example facsimile (consistent with how it
  demonstrates every other field) or omits this section since it's not a
  real tune. Flagging rather than deciding — same open question as the
  subtitle field's legend-card example text.
- **Cards with no recipe at all.** Some catalog entries are livery-only with
  no tune attached. The whole section — facsimile and auto-list both —
  should be absent, not rendered empty. Gate on whether `recipe` exists at
  all, not just on whether `adjustments[]` is non-empty.

## 4. Known risk not addressed by this pass

The row layout (label, two end-fields, a track, a floating badge) is dense —
four interactive elements across one row. `CLAUDE.md` already lists mobile
layout as "not yet designed or implemented" for the rest of the app; this
component is likely to be one of the harder pieces of that work, not an
easier one. Worth treating as its own pass rather than assuming the current
layout degrades gracefully.

## 5. Checklist before building

- [ ] Confirm only non-empty rows get persisted (Section 1) — no placeholder
      rows for locked/unused tabs
- [ ] Build the engaged/disengaged confirm dialog for Define Stock, reusing
      `ExitConfirmModal` (Section 2) — decide the color convention deliberately
- [ ] Decide the legend card's treatment (Section 3)
- [ ] Gate the whole section on `recipe` existing, not just `adjustments[]`
      being non-empty (Section 3)
- [ ] Treat mobile as a separate, explicit pass — don't assume this layout
      survives a narrow viewport unchanged (Section 4)
