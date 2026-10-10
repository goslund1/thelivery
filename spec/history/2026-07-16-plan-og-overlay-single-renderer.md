# OG Overlay Studio — Single-Renderer Addendum

Addendum to `plan-og-overlay.md`. Covers one architecture decision: how the OG Maker's live editing preview relates to the server-side PNG compositor, and why. Meant to be read alongside the original plan, not replace it — section references below point back to the original doc's numbering.

---

## The problem

As originally scoped, the OG Maker modal (Section 1) renders its canvas as absolutely-positioned DOM/CSS elements over a background photo — a live, interactive approximation of the final card. The actual `og:image` PNG (Section 3) is produced separately, server-side, in Rust, using the `image` crate for compositing and `ab_glyph` for text rendering.

That's two independent implementations of the same layout logic — one in CSS/DOM, one in Rust image compositing — built at different times by different code, trying to agree on stamp position, auto-width text behavior, rail-snapping, and font rendering. Nothing enforces that they actually match. If they drift (different font metrics, different auto-width math, different edge-clipping behavior), someone builds a preset that looks right in the editor and wrong in the actual Discord/Reddit unfurl, and there's no structural reason that gets caught early.

## The recommendation

Don't build two renderers. Have the live preview call the real backend compositor instead of approximating it in the browser.

Concretely:

- While a text box is actively being dragged, the canvas shows a lightweight local overlay — just a box and label indicating position, no attempt at real typography or compositing. This is for pointer responsiveness only.
- On drag-stop (debounced ~150–300ms after the pointer settles, or on drop), the client sends the current in-progress config to the backend and swaps in the returned PNG as the visible preview.
- Because the preview image and the final `og:image` are produced by the same compositor function, there is no visual drift possible between "what the editor shows" and "what ships." There's nothing left to keep in sync — it's one implementation, called in two contexts.

This replaces the DOM/CSS visual layer entirely for anything that affects final appearance (fonts, text box rendering, photo compositing). The DOM layer's only remaining job is capturing interaction — drag position, click-to-select, handle state, transform controls — not rendering the final look.

## New backend requirement: a preview endpoint

Section 3 of the original plan specifies `GET /share/:id/card.png`, which reads a **persisted** `share_overlay_config` off a saved card. That's correct for the public-facing `og:image` route, but it doesn't cover live editing — while someone's dragging boxes around in the OG Maker, that config hasn't been saved yet and may never match what's on the card.

Add a second route for this:

**`POST /share/preview`**
- Accepts the same config JSON shape already defined in Section 3 (`photo_id`, `logo_visible`, `text_boxes[]`) directly in the request body — not tied to any saved card.
- Internally calls the exact same compositor function as the persisted route. This must be shared code, not a second implementation, or the whole point of this addendum is lost.
- Returns the rendered PNG.
- Does not need the same cache treatment as the public route — config changes on nearly every request during an editing session. A cache keyed on a hash of the request body is fine if it helps with repeated states (undo/redo landing back on a prior config), but it's not load-bearing the way the public route's cache is.

## Build order impact

The original build order puts full PNG composition at step 6, after the OG Maker modal (step 4) and share flow integration (step 5) are already built. Under this recommendation, that ordering doesn't work — the modal can't render a live preview without a working compositor, so the compositor can't be the last thing built.

Revised sequencing:

1. Stubbed PNG route (unchanged) — proves routing/plumbing end to end.
2. `og_presets` table + CRUD (unchanged).
3. **Real compositor logic, minimally** (pulled forward) — hero photo + at least one text box style, wired to both `GET /share/:id/card.png` and the new `POST /share/preview`.
4. OG Maker modal — built against the now-working preview endpoint from the start, not against a DOM approximation.
5. Share flow integration (unchanged).
6. Full compositor polish — remaining text box styles, edge cases, logo slot.

The stub-first instinct from the original plan still holds and matters more now, not less: get the route returning *something* before building real compositing, then get real compositing working before building draggable UI on top of it.

## Alternatives considered

**Headless browser screenshot** (Puppeteer/Playwright rendering the same HTML/CSS used in the editor, server-side) — would avoid the debounce-and-round-trip feel entirely by keeping the DOM as the single source of truth in both places. Not recommended: pulls in a heavy browser-automation dependency and its own per-render resource cost, for a problem the debounced-Rust-call approach already solves without new infrastructure.

**Satori** (the layout engine behind Vercel's `next/og`) — purpose-built for exactly this class of problem: deterministic, constrained (flexbox-only) layout, no browser needed, fast. The catch is it's a JS/TS library, so using it would mean standing up a second backend runtime — a small Node service — alongside the existing Rust backend, just for this one feature. Not recommended right now, specifically to avoid doubling the number of runtimes/deploy targets for a single-purpose endpoint. Worth revisiting only if the debounced-preview approach feels bad in practice once built.

## Clarifications on the DOM interaction layer

**Transform visual feedback during gesture** — "interaction only" means the DOM layer is responsible for live visual feedback of the full transform state while a gesture is in progress, not just position. If someone is rotating a box, the placeholder must visually rotate in real time — the compositor fires on gesture-end, not during. A CSS `transform` on the drag placeholder (translate + rotate + skewX) fed by the same values being tracked in the composable is the right approach. The DOM layer shows the transform happening; the compositor confirms the final result.

**Undo/redo** — the addendum mentions a config-hash cache helping with "undo/redo landing back on a prior config." That's worth making explicit: undo/redo in the OG Maker is **in scope** as a basic quality-of-life feature (placing boxes and discovering you hate where they are is the normal editing loop), but it doesn't need a formal undo stack library. A simple array of config snapshots pushed on each committed state change (drag-stop, transform-settle, style change, text edit) with a cursor is sufficient. The config-hash cache on `POST /share/preview` then naturally serves repeated undo/redo states without re-compositing. Scope: 10–20 snapshot depth, no branching history.

## Tradeoff being accepted

A short, debounced lag between releasing a dragged text box and seeing the true rendered result, instead of an instant but potentially inaccurate DOM approximation. Also, more server load during active editing sessions than a pure client-side preview would generate — bounded to OG Maker usage, not the public `og:image` traffic path.

Accepted explicitly: the guarantee that the editor and the shipped card can never visually diverge is worth a few hundred milliseconds of lag on drag-release.

## One-line summary

Kill the second renderer. The OG Maker's live preview should call the real Rust compositor (via a new `POST /share/preview` endpoint sharing code with the persisted `GET /share/:id/card.png` route) instead of approximating the look in DOM/CSS, and the compositor needs to be built early enough to power the editor itself — not deferred to the last build step.
