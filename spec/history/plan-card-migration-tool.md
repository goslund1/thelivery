# Card Migration Tool — Plan

A tool for bringing existing cards into alignment with schema and data model
changes introduced after the original extraction from the legacy HTML file.
Needed because old cards were extracted as-is and some fields have evolved —
most notably the adjustment rows, which moved from free-text `{ name, description }`
to the structured `AdjustmentRow` format with typed sliders.

---

## The problem

The original extraction produced cards with whatever shape the legacy HTML had.
As the data model has grown — structured adjustments, canonical upgrade categories,
per-card display options, theme overrides — old cards fall behind. Running the
live site with mixed-schema cards means:

- Auto-populate (upgrades ↔ tuning link) can't work on old-format adjustment rows
- New UI features that depend on structured fields silently no-op on old cards
- The legend card (which IS structured) drifts from actual card content over time

Manual editing in the UI fixes one card at a time but doesn't scale. The migration
tool provides a structured, reviewable path to bring any card up to date.

---

## Scope

The tool is an **admin-only panel** inside `UserSettingsModal` (a new "Migrate"
tab alongside Password / Create User / Admin). It operates on individual cards
or all cards at once, with a preview-before-commit workflow.

It does not run automatically. Every migration is an explicit admin action.

---

## Migration operations

### 1. Upgrade category normalization
Rename non-canonical category strings and reassign parts to the correct
category. Runs against the canonical mapping defined in `plan-upgrades-tuning-link.md`.

Input: card with `"Platform & Handling": ["Race Springs & Dampers", "Race Brakes"]`
Output: card with `"Springs & Dampers": ["Race Springs & Dampers"]`, `"Brakes": ["Race Brakes"]`

Safe to run multiple times — idempotent.

### 2. Adjustment row migration (free-text → structured)
The hardest migration. Old format:
```json
{ "name": "Front anti-roll bar", "description": "softened two clicks from stock" }
```

Structured format:
```json
{
  "tab": "arb", "group": "Front Anti-Roll Bar", "key": "arbFront",
  "label": "Front", "unit": "", "min": 1, "max": 65, "stock": 5.0,
  "value": 5.0, "step": 0.5
}
```

**This cannot be done automatically** — the old format has no numeric values,
no min/max, no tab assignment. The migration tool flags these rows as
"needs manual entry" and presents them for the admin to fill in one by one.
The tool provides the old name/description as context and a form to enter
the structured values.

### 3. Display fields backfill
Add `themeOverride: null` and `display: { badges: {...}, defaultExpanded: "none" }`
to cards that predate Module B (per-card theme override). Safe to auto-run —
these are nullable defaults that change nothing visually.

### 4. Legend sync check
Compare each card's section structure against the legend card (card 000) and
flag any sections that exist on the legend but are absent on the card, or
vice versa. Does not auto-fix — just surfaces the diff for review.

---

## UI

New "Migrate" tab in `UserSettingsModal`, auth-required.

### Card selector
Dropdown or list of all cards. "All cards" option runs the safe auto-migrations
(operations 1, 3) across every card in one pass. Operation 2 always runs
per-card due to the manual entry requirement.

### Migration status per card
For each card, show which operations are needed:

| Operation | Status |
|---|---|
| Upgrade categories | ✅ Clean / ⚠ Needs normalization |
| Adjustment rows | ✅ Structured / ⚠ N free-text rows need manual entry |
| Display fields | ✅ Present / ⚠ Missing — auto-fixable |
| Legend sync | ✅ In sync / ⚠ Diff found |

### Preview before commit
For auto-migrations, show a diff of what will change before writing. Admin
confirms, then the tool PUTs the updated card to the API.

For adjustment row migration, a form appears for each free-text row:
- Shows the original `name` and `description` as read-only context
- Fields: tab (select), group, label, unit, min, max, stock, value, step
- On save, replaces the free-text row with the structured row in the card

### Bulk safe-run
"Fix all auto-fixable issues" button runs operations 1 and 3 across all cards,
shows a summary of what changed, and writes in one batch.

---

## Backend

No new endpoints needed beyond what exists. The migration tool uses:
- `GET /api/cards` — load all cards
- `PUT /api/cards/:id` — write migrated card back

The existing save mechanism handles it. The tool is purely a frontend
admin workflow that reads, transforms, and re-saves card data.

---

## Implementation phases

### Phase 1 — Upgrade category normalization (auto)
- Build the normalization transform function
- Add Migrate tab to UserSettingsModal with card status display
- Wire "Fix upgrade categories" action per card and bulk

### Phase 2 — Display fields backfill (auto)
- Add to the same bulk-run as Phase 1
- Trivial once the UI shell exists

### Phase 3 — Adjustment row migration (manual)
- The form for entering structured values for each free-text row
- This is the bulk of the UI work
- Consider showing the legend card's adjustment schema as a reference
  while filling in values for a card

### Phase 4 — Legend sync check
- Read-only diff view — no auto-fix
- Low priority; useful for long-term maintenance

---

## Open items

- **Chassis category** resolution (from `plan-upgrades-tuning-link.md`) must
  happen before Phase 1 runs, or the normalization will be incomplete
- **Adjustment row defaults**: when manually entering a structured row,
  what are sensible default min/max/step values per tab? Define a defaults
  table per tab to pre-fill the form and reduce manual entry burden
- **Undo**: the tool writes directly to the API with no undo beyond the
  existing snapshot/discard mechanism in edit mode. Consider whether a
  "dry run" export (download migrated JSON without writing) is worth adding
  as a safety valve
