# License / Terms Page — Draft

Sketch for the item flagged in `SEO_AND_ATTRIBUTION_SPEC.md` ("License/terms page or footer text — nothing visible yet"). License choice: **CC BY-NC 4.0** — real, recognized, allows sharing/reference with credit, blocks commercial use.

## Where it lives

- Dedicated page at `/license` (or `/terms` — pick whichever matches existing route naming).
- Linked from the site-wide footer on every page, not just the card pages.
- Referenced by the new `"source"` attribution field being added to `GET /api/cards` (Goal 2, item 2 in the SEO spec) — the field text can point here.

## Footer line (site-wide, every page)

```
© [Jason] · Builds & tunes licensed CC BY-NC 4.0
```

"CC BY-NC 4.0" links to `/license`.

## Page copy draft

---

### Builds, Tunes & License

Everything here — the liveries, the tunes, the write-ups — is work I've put together over about five years across Forza Horizon 5 and 6. If you find something useful, grab the share code, try the tune, reference it in your own build. That's what it's here for.

If you're pulling it into a collection, a guide, or anything you're publishing elsewhere, credit **The Livery** and link back. That's the one ask.

**This site's original content — liveries, tunes, and write-ups — is licensed under [Creative Commons BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/).**

In plain terms:

- You can share, remix, or reference anything here.
- Credit The Livery (and link back) when you do.
- Don't use it commercially — no reselling tunes, no monetized collections built on this data — without asking first.

**What this doesn't cover:** the cars themselves, in-game assets, and manufacturer likenesses belong to Playground Games, Microsoft, and the respective car manufacturers. This site is fan-made content built on top of Forza Horizon, not a claim of ownership over the game or the cars in it. Where car reference data comes from an external database, that source is credited on the relevant pages per its own terms.

---

## Notes for implementation

- CC BY-NC 4.0 badge image/link available directly from creativecommons.org — no need to host it ourselves.
- The "external database" credit line in the draft should match however the existing carDB "sourced by" credit is currently worded, so the two don't contradict each other stylistically.
- Once this page exists, the `"source"` field being added to the API response (per the SEO spec's priority #3) can literally just be the `/license` URL, or a short string plus that URL — Jason's call on exact wording, this doc doesn't lock that in.
- Open question carried over from the SEO spec: does `GET /api/cards` need any access control now that attribution is handled via a field rather than a gate? Not resolved here — separate decision.
