# thelivery to the Projects standard: work order for CC

From App Claude, 2026-10-09, after Jason settled the Projects standard
(`~/Projects/standards/STANDARD.md`; decisions D2 and D3 there). Read
`~/Projects/CLAUDE.md`, that file and `standards/NEW-PROJECT.md`
("Bringing an existing project to the standard") first.

**Do not start until the mobile pass is finished.** That work order
names `docs/` paths (`docs/plan.md`, `docs/shakedown.md`,
`docs/mobile-pass/`), and this one renames the folder.

Docs only: no code changes. Relocation, not rewriting: the wording of
each gotcha and each AAR is the valuable part. Open in plan mode,
state the mapping back, and wait for Jason's go.
**This repo's commit rule stays as it is: commit and push only when
asked**, because a push to `main` deploys the live site. Suggest a
commit at the end of each step and let Jason say when.

| Step | Do | Done when |
|---|---|---|
| 1 | Freeze `docs/plan.md` and `docs/shakedown.md` as they stand into `docs/completed/verbatim/`, byte for byte. | `cmp` confirms both. |
| 2 | Rename with `git mv`: `docs/` to `spec/`, then `spec/completed/` to `spec/history/`. In the same commit change `.gitignore`'s `docs/mobile-pass/**/*.png` to `spec/mobile-pass/**/*.png`: the folder moves with its untracked screenshots, and the old line would no longer hide them. | `git status` shows renames and the one `.gitignore` line; no `.png` is listed. |
| 3 | Move the dated files still in `spec/` into `spec/history/`: the four `aar-2026-07-17*.md`, `shakedown-2026-07-17.md`, and any handoff. Rename history files that lack a date in front to `YYYY-MM-DD-name.md`, using the date in the file or its last commit. | Every file in `spec/history/` starts with a date. |
| 4 | Write `spec/history/README.md`: one row per file (what it is, what superseded it). About 40 rows. | The checker's history check passes. |
| 5 | Write `spec/status.md` (150 lines at most): what is live, what is in flight, what is next, the pre-launch list's state. Draw it from `plan.md`'s "Active" section and the newest AARs. Present tense. | A new session can tell where things stand from this file alone. |
| 6 | Split `plan.md`. Open items (Active, Maintenance, Pre-launch checklist, Deferred) become `spec/backlog.md` entries, one line each with IDs from `B-1`, and a "Next free ID" line. Every struck-through or finished item, and the whole "Recently completed" section, moves word for word into dated history files, grouped by the date each carries. Then `plan.md` holds only founding decisions, or is removed if nothing is left. | `backlog.md` has no finished item; nothing from the frozen copy is missing (check by script against step 1's copy). |
| 7 | `shakedown.md`: keep as the hands-on checklist. If its rows do not fit the standard's shape (one line, 450 characters, `- [ ]`), say so in the plan and let Jason choose between reshaping and leaving it. | Jason has chosen. |
| 8 | Update every path: `CLAUDE.md` ("See also" becomes a docs table naming every live spec; `docs/…` becomes `spec/…`), both skills, `README.md`, and any code comment naming `docs/`. Add the line about `Drop Offs/` and the clear. | A grep for `docs/` in live docs and skills finds nothing stale. |
| 9 | Add `tools/docs-check.sh` from `standards/template/tools/`, and `Drop Offs/` to `.gitignore` (it shows as untracked today). | `tools/docs-check.sh` passes. |
| 10 | Write the day's history file and its README row; freeze this work order and both mobile work orders in `spec/history/verbatim/`. | The checker passes; Jason is offered the commit and push. |

## The screenshots (settled 2026-10-09)

`docs/mobile-pass/` had its screenshots committed and pushed during
Phases 0 and 1 (the mobile work order asked for the "before" set to be
saved there; that instruction was App Claude's). Claude Code stopped
tracking them the same evening (`91b719a`): `audit.txt` and
`audit.json` stay in the repo and the pictures stay on the Mac. What is
left for this work order is step 2's `.gitignore` line. The pictures
already pushed are still in the repo's history; removing them rewrites
history on a repo that deploys, so leave that unless Jason asks.

Also before starting: the bottom bar, star and Playing pill work order
in this folder comes first. It names `docs/` paths too.
