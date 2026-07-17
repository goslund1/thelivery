# Plan: Suggestion Viewer

## What it is

A dedicated full-screen modal for reviewing incoming tune suggestions. Primary
use case: read the submitted adjustments tab by tab, dial them into the game on
the controller, then dismiss. Liking a suggestion parks it for later; Apply is a
low-priority edge case (more likely a new card gets made).

---

## Entry point

**SideBug filters toggle button** — gets a count badge showing the number of
pending (unreviewed) suggestions. Visible before the panel opens; unambiguous
because suggestions are the only dynamic count in the list.

**Inside the filters panel** — a "Tune Suggestions (N)" row. Clicking it opens
SuggestionViewer. Admin-only: row only renders when the user is authenticated.

---

## SuggestionViewer modal

Full-screen shadowbox, same glass surface pattern as EditCardModal (Teleport to
body, backdrop blur, centered panel).

### Header
- Card name + make/model (looked up from the cars store via the suggestion's `card_id`)
- Submitter credit (Gamertag / Discord / etc. as entered in the suggest form)
- Submission date

### Suggestion selector
Dropdown at the top of the modal. Two modes toggled by a small tab or segmented
control:
- **Pending** — new, unreviewed suggestions (default)
- **Liked** — saved for later consideration

Each entry shows: Card name · Submitter · Date.
On Dismiss or Like, the dropdown automatically advances to the next item in the
current list.

### Adjustments view
Read-only TuningAdjustments rendered with the suggestion's adjustment rows.
- All sliders visible and tab-grouped exactly as normal
- **Diff highlighting**: rows where `suggestion.value ≠ card's current value`
  are highlighted — same visual treatment as Show Stock diff comparison
- No flush/emit; sliders are display-only (new `readOnly` prop on TuningAdjustments)

### Actions (three buttons)
1. **Like** — marks suggestion as `liked` in the DB; moves it out of Pending into
   Liked; auto-advances to next Pending item
2. **Apply to Card** — low-priority edge case. Merges suggestion's adjustment rows
   into the card's recipe, auto-credits the submitter in the card (TBD field),
   marks card dirty. Discuss exact credit placement when it becomes relevant.
3. **Dismiss** — DELETEs from DB; auto-advances to next Pending item. Use when
   you've read the values and entered them in-game, or when the suggestion isn't
   useful.

---

## Backend changes

### Migration
Add `status TEXT NOT NULL DEFAULT 'pending'` to the `suggestions` table.
Values: `'pending'` | `'liked'`.

### New endpoint
`PATCH /api/admin/suggestions/:id` — toggles status between `pending` and
`liked`. Admin-auth required (same as existing suggestion endpoints).

### Existing endpoints (no change needed)
- `GET /api/admin/suggestions` — already returns all rows; frontend filters by
  status client-side
- `DELETE /api/admin/suggestions/:id` — Dismiss action (unchanged)
- `POST /api/suggestions` — visitor submit (unchanged)

---

## Frontend changes

### `backend/migrations/` — new migration
Add `status` column to `suggestions` table.

### `frontend/src/api.ts`
- Update `Suggestion` type: add `status: 'pending' | 'liked'`
- Add `likeSuggestion(id: string): Promise<void>` — PATCH endpoint

### `frontend/src/components/TuningAdjustments.vue`
- Add `readOnly?: boolean` prop
- When true: disable flush, suppress all dialogs, keep sliders visual-only
- Diff highlighting: currently compares `value` vs `stock`; for suggestion view
  we need to compare suggestion `value` vs card's current `value`. Pass the
  card's current adjustments as a `baselineAdjustments` prop; when set, diff
  against those instead of `stock`.

### `frontend/src/components/SuggestionViewer.vue` (new)
- Full-screen modal, Teleport to body
- Loads suggestions on open via `api.adminListSuggestions()`
- Pending / Liked tab switch
- Suggestion dropdown
- Header (card name, make/model, credit, date)
- Read-only TuningAdjustments with baseline diff
- Like / Apply / Dismiss buttons
- Auto-advance on Like or Dismiss

### `frontend/src/components/SideBug.vue`
- Badge on the filters toggle button: pending suggestion count
- New "Tune Suggestions (N)" row in the filters panel (admin-only)
- Clicking the row emits an event or sets a flag to open SuggestionViewer

---

## Implementation order

1. DB migration + PATCH endpoint (backend)
2. `api.ts` type + method updates
3. `TuningAdjustments` readOnly + baselineAdjustments props
4. `SuggestionViewer.vue` shell (modal, dropdown, header, Like/Dismiss wired up)
5. Wire TuningAdjustments into SuggestionViewer
6. SideBug badge + filters row entry point
7. Apply to Card (defer until needed)

---

## What's already built

- `POST /api/suggestions` + rate limiting
- `GET /api/admin/suggestions` + `DELETE /api/admin/suggestions/:id`
- Admin panel "Tune Suggestions" list (title, card ID, date, credit, Dismiss)
- Suggest bar + submit modal in TuningAdjustments (visitor-facing, fully wired)
