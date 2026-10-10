# Plan: Submit Tune Feature

## What's established

- Visitor adjusts sliders on a card → prompt appears to submit their tune
- Submission lands in an admin-only queue ("the pile") with a notification badge
- Jason reviews, optionally tests, optionally applies to the card or creates a variant
- Submitter can optionally leave contact info — not required
- Only Jason sees submissions; spam/abuse is a real risk

## Current implementation problem

The trigger fires on the first non-stock slider move. But cards already show Jason's
modifications FROM stock — so sliders are always non-stock on load. This causes the
suggest bar to fire immediately on page load for any card with tuning data.

The trigger logic needs a redesign before anything else can ship.

## Open questions (to resolve next session)

1. **Trigger redesign** — if not "first non-stock move," what is it?
   - Explicit: visitor hits a "Submit my tune" button they choose themselves?
   - Threshold: after N slider moves from the card's current (non-stock) values?
   - Something else?

2. **Spam prevention** — what was settled on?
   - Rate limiting (per IP)?
   - Honeypot field?
   - Accept-and-curate (rely on admin review, no technical gate)?
   - Require contact info as a friction mechanism?

3. **The pile UI** — where does the notification live and what does the queue view look like?
   - Badge on the SideBug / admin panel?
   - Dedicated modal or tab in UserSettingsModal?

4. **Applying a submission** — when Jason accepts one, what happens?
   - Overwrites the card's current tuning values?
   - Creates a named variant?
   - Manual copy-paste into the card?

## Dependencies

- **Car Identity** must be built first — a submitted tune needs to be anchored to a
  specific car (make/model/year/game), not just a card ID. See `plan-car-identity.md`.
