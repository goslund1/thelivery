# SEO & Attribution Spec

## Context — how we got here

This came out of a conversation that started off-topic from the site itself: Jason was researching the FH6 tire meta after the Series 2 patch (June 15, 2026) nerfed Drag Tires' off-label cornering grip, and comparing two of his own C-class rally builds (a Celica that underperformed vs. a second car/tune that took 1st) to figure out what made the difference.

That led to a question: could Claude, in the future, just look at thelivery.silverleaf.services directly to pull tune/stat data for comparisons like this, instead of Jason re-typing or screenshotting it each time — assuming tunes are entered as structured data via the tuning facsimile fields on a card.

The answer was yes in principle, with a caveat: since the frontend is a Vue SPA, a plain fetch only returns an empty shell — reading real data requires either hitting Geoff's API directly or rendering the page with a JS-capable tool (e.g. Claude in Chrome).

Jason then asked whether that same rendering behavior was an accidental form of IP protection against scrapers. Short answer: not really — it only blocks lazy tools, not anything that actually renders the page (which includes every real visitor and any serious scraper). That distinction matters to Jason because of the precedent he already set with the car database: he used a sourced dataset for the site but insisted on adding a "sourced by" credit line, and he'd like any of his own tune/build data to carry similar attribution if it gets pulled into someone else's collection — while still being independently fine with people gathering his (currently tiny) sample of builds.

Separately, Jason confirmed he *does* want the site to be broadly searchable — the goal is that someone searching "best tune for FH6 Ferrari Pista" finds his card (once he has one), sees the share code, and can learn from the build. So there are two related but distinct goals here: **make the content discoverable to search engines**, and **make sure any data that gets scraped or aggregated carries attribution back to the site**. These aren't in tension with each other, but they are solved with different mechanisms, laid out below.

This spec is written now, before Jason pivots from "building the app" to "ongoing content — adding more cars," specifically so these considerations can be evaluated (some may already be handled) before the card catalogue scales up.

## Goal 1: SEO / discoverability

The core problem: because the frontend is a Vue SPA, search engines may not reliably see car names, tune summaries, or share codes in the raw HTML — they only appear after JavaScript executes, which not all crawlers handle consistently, especially for a small/low-authority site.

Recommendations, roughly in priority order:

1. **Unique per-car `<title>` and meta description.** Each card page needs its own title/description reflecting the car and game — e.g. "Ferrari 296 GTB Pista Tune — Forza Horizon 6 Build & Share Code | The Livery" — rather than one generic site-wide tag. This is likely the single biggest lever for exact-match searches like "fh6 ferrari pista tune."
2. **Prerendering or SSR for car pages.** Doesn't require a full framework rewrite — prerendering each card route to static HTML at publish time, or a lightweight SSR fallback specifically for bots, would put car name/tune summary/share code into the raw HTML instead of relying on JS execution. This is the one item worth a direct conversation with Geoff, since it touches the Rust/Axum layer.
3. **Human-readable URL slugs per car** — e.g. `/cars/fh6/ferrari-296-gtb-pista` instead of an ID or UUID — for better search term matching.
4. **A `sitemap.xml` that updates as cards are added**, so new cars get discovered without waiting on organic crawl/link discovery.

## Goal 2: Attribution when data is scraped or aggregated

Jason isn't trying to prevent scraping outright — he's fine with his (currently small) sample of builds being gathered elsewhere. The ask is that if it happens, credit travels with the data, consistent with how he already requires a "sourced by" credit on the car database data he used.

Recommendations:

1. **Selective `robots.txt`.** Allow standard search crawlers (Googlebot, Bingbot) so the site stays indexable, while explicitly disallowing known AI/scraper bots (GPTBot, CCBot, ClaudeBot, Google-Extended, Bytespider, etc.). Not legally enforced, but respected by reputable crawlers and creates a clear paper trail otherwise.
2. **Embed attribution directly in API responses, not just the rendered page footer.** A page-footer credit only survives if someone scrapes the rendered HTML. If the API returns raw JSON (which any programmatic scraper would prefer to hit directly), include a source/attribution field in the payload itself — e.g. `"source": "thelivery.silverleaf.services"` — so the credit travels with the data even if it's pulled via the API and never rendered.
3. **Explicit license/terms.** A visible license statement (e.g. CC BY-NC-ND, or plain "builds © Jason [name], no reproduction without credit") in the footer and/or a `LICENSE` or terms page gives actual standing if the data is lifted wholesale without the required credit — versus no stated terms today, which is murkier.
4. **Rate limiting on the API, separate from the rendered page.** If the API has no throttling, a scraper can hit it directly and skip rendering entirely, pulling structured data far faster than any real visitor would. Basic rate limiting raises the bar without affecting normal use.

## Open questions / to verify before scaling up card content

- Which of the above, if any, are already in place in Geoff's Rust/Axum backend or the current deployment config (robots.txt, sitemap, meta tags, rate limiting)?
- Does the current API expose card/tune data as open JSON with no auth, or is there already some access control?
- Is prerendering/SSR feasible within the current Vite/Vue setup without a major rework, or does it need to be scoped as a larger backend conversation with Geoff?
- Where should the license/terms text live — footer line, dedicated page, or both?

None of this blocks current work — it's meant to be reviewed once, ideally before the card catalogue scales past the placeholder-data phase, so slug structure and meta-tag patterns don't need to be retrofitted across a large number of cards later.
