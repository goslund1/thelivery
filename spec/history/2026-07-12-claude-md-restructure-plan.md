# Task: Split CLAUDE.md into three right-sized artifacts

## Ground rules — read first

- **This is a documentation reorganization, not a rewrite.** Move and regroup
  existing prose; don't paraphrase, "improve," or summarize it. The value of
  this file is the specific, hard-won detail already in it (exact gotchas,
  exact reasoning) — preserve wording as-is wherever possible.
- **Do not touch any code, config, or non-markdown file.** Scope is limited to
  creating/editing the three markdown files named below.
- **Enter plan mode before making any edits.** State the exact section-by-
  section mapping you intend to use (see below) and get an explicit go-ahead
  before writing anything.
- **Do not commit.** Leave changes staged/unstaged for review; commit only
  when asked, per this repo's existing rule.
- **If any section doesn't cleanly fit the categories below, stop and ask**
  rather than guessing which bucket it belongs in.

## Why

The current root `CLAUDE.md` is 271 lines and loads in full on every session
regardless of what the session is about. Per Anthropic's own guidance,
adherence to instructions degrades non-linearly with length — rules start
getting lost well before 271 lines. The file also mixes three genuinely
different kinds of content (always-relevant conventions, situational
subsystem patterns, and a changelog-style feature log) that don't need to be
loaded together. Splitting by relevance, not by rewriting, should recover
adherence without losing any of the captured knowledge.

## Target structure

### 1. Trimmed `CLAUDE.md` (repo root) — stays always-loaded

Keep, in current form:
- `## What this is`
- `## The data model is the center of everything` (full Card/Section schema —
  this is foundational and referenced constantly, keep it whole)
- `## Running locally`
- `## Build / typecheck / verify`
- `## Backend (backend/)`
- `## Frontend (frontend/src/)` — but **only** the `### Stack & layout`,
  `### CSS — important`, and `### Component tree` subsections
- `## Images`
- `## Deployment`
- `## Conventions & rules`

Target: roughly 120–150 lines. Add a short `## See also` note near the top
pointing to the two new files below, so Claude knows they exist even though
they aren't auto-loaded in every session.

### 2. New skill: `.claude/skills/frontend-patterns/SKILL.md`

Move these `### ` subsections out of the current `## Frontend` block, in full:
- `### Float panel system` (includes the two-class naming convention and the
  embedded GOTCHA about `float_*_panel` placement)
- `### Edit mode + per-card persistence`
- `### Slideshow` (includes the thumb-rail `scrollIntoView` GOTCHA)
- `### Custom tooltips`
- `### Card list rendering` (includes the "why not virtual scroll" reasoning
  and the `content-visibility` note for future scale)

Also move, from further down the current file, the larger named-feature
entries that are really subsystem docs, not status updates — use judgment,
but candidates include the **Theme builder / ColorPicker / DrawerPanel /
two-surface drawer design principle** block and the **Tune Suggestion
Viewer** block. These describe *how a pattern works*, which is what a skill
is for — not *whether it's done*, which belongs in file #3.

Frontmatter:
```yaml
---
name: frontend-patterns
description: Vue component patterns and subsystem behavior for the Livery Catalog frontend — floating panels, edit-mode persistence, slideshow, tooltips, card list rendering, drawer/theme system. Load when working on these components or extending similar patterns.
---
```

### 3. New skill: `.claude/skills/frontend-gotchas/SKILL.md`

Move these `### ` subsections out in full, unchanged:
- `### CSS overflow-x: auto implies overflow-y: auto`
- `### vue-tsc gotcha`
- `### Imports must be at the top of <script setup>`
- `### e.preventDefault() on mousedown blocks focus`
- `### focusedKey ≠ DOM focus (TuningAdjustments)`
- `### Props are a separate reactive graph from the Pinia store`
- `### Multi-column layout decision framework`

Frontmatter:
```yaml
---
name: frontend-gotchas
description: Known Vue/TypeScript/CSS pitfalls specific to this codebase (vue-tsc quirks, focus-handling traps, reactive-graph divergence, overflow CSS spec behavior). Load before debugging unexpected frontend behavior or writing new interactive components.
---
```

### 4. `docs/status.md` (or fold into existing `docs/plan.md`)

Move wholesale:
- The entire `## Feature status` section — both `### Shipped and working` and
  `### Pending / in progress`

If `docs/plan.md` already serves this purpose for pending work, add the
Shipped content there under its own heading rather than creating a redundant
file — check `docs/plan.md` first and follow whatever pattern already exists.

## After moving content

1. Grep the new trimmed `CLAUDE.md` for any dangling references to moved
   subsections (e.g. "see Float panel system above") and fix the pointer to
   name the new file instead.
2. Show a summary: final line count of each of the four files, and confirm
   nothing from the original 271 lines was dropped rather than relocated.
3. Do not run `npm run build` or `cargo build` — this is a docs-only change
   and shouldn't touch anything those commands check. If you find yourself
   wanting to "fix" or "improve" something in the code while doing this, stop
   and flag it separately instead of doing it inline.
