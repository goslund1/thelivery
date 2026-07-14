# Reddit Sharing — Status & Approach

## Current approach: pre-fill link (implemented 2026-07-13)

Clicking "Post to Reddit" in the ShareModal opens a new tab to Reddit's own submission form, pre-filled with the share URL and a generated title. The user picks their subreddit and clicks Post on Reddit's page. No backend, no credentials, no approval required.

**Title format:** `{cardName} — {firstCarName} | Share code: {shareCode}` (editable in the modal before opening)

**URL opened:** `https://www.reddit.com/submit?url={encodedShareUrl}&title={encodedTitle}`

---

## Why the direct OAuth approach is off the table

Reddit ended self-service API access in **November 2025** via their "Responsible Builder Policy."

- The old `prefs/apps` flow (create a script app, get credentials instantly) is gone
- All new OAuth tokens require manual review via a support ticket
- Personal/hobby projects are explicitly excluded from approval — Reddit favors commercial or clearly-scoped use cases
- There is no bypass; the CAPTCHA is irrelevant — the policy gate is server-side

**Source:** [Reddit Killed Self-Service API Keys](https://molehill.io/blog/reddit_killed_self-service_api_keys_your_options_for_automated_reddit_integration) · [Reddit's 2025 API Crackdown](https://replydaddy.com/blog/reddit-api-pre-approval-2025-personal-projects-crackdown)

Existing pre-Nov-2025 credentials still work, but we don't have any. New ones won't be granted.

---

## What the original plan called for

The original plan (`git log` has the full doc) called for script-type OAuth (password grant), a token cache in `AppState`, and a `POST /api/admin/reddit/post` endpoint. That flow is sound architecturally — if Reddit ever opens personal API access again, it's the right approach. But it can't be built today.

---

## If this changes

Reddit is pushing **Devvit** (apps that run on Reddit's own infrastructure) as the approved alternative for developers. That's a fundamentally different model (no external server, Reddit-native) and not a fit for thelivery's architecture.

If direct API posting ever becomes viable again, the implementation plan is:
- 5 env vars: `REDDIT_CLIENT_ID`, `REDDIT_CLIENT_SECRET`, `REDDIT_USERNAME`, `REDDIT_PASSWORD`, `REDDIT_USER_AGENT`
- Token cache in `AppState` (`tokio::sync::Mutex<Option<RedditToken>>`, 1hr TTL)
- `POST /api/admin/reddit/post` with subreddit allowlist, `kind=link` post
- Replace the pre-fill button in `ShareModal.vue` with the direct-post UI (subreddit dropdown, post button, success/error state)
