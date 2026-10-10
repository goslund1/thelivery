# CC Code Evaluation Audits

These are evaluations *of the code itself* — quality, health, risk — as
opposed to the process audits (scope, verification, prior-art) in the other
file. Each is a real, named category in software engineering; giving CC the
name plus the prompt gets you a sharper result than describing it in your own
words each time, because the name pulls in the right frame of reference.

---

## Code smell audit

**What it catches:** duplicated logic, overly long functions, tight coupling,
magic numbers, god objects, dead branches. Style/structure — not correctness.

```
Run a code smell audit on [file/directory]. Look specifically for:
duplicated logic, functions doing too many things, magic numbers/strings
that should be named constants, tight coupling between modules that should
be independent, and dead or unreachable branches. Rate severity for each
finding (cosmetic / worth fixing / should fix before it spreads).
```

---

## Correctness / logic trace audit

**What it catches:** the thing smell audits can't — code that's clean but
wrong. Pair this with #1, never substitute one for the other.

```
Pick the core function(s) in [file]. For each, trace execution against 2-3
realistic inputs including at least one edge case, and state the actual
output. Don't evaluate whether the code "looks right" — only whether the
traced output matches the intended behavior.
```

---

## Architecture / consistency audit

**What it catches:** places where a pattern was implemented one way in one
spot and a different way elsewhere — the kind of drift that accumulates
silently across many small fixes.

```
Compare how [pattern/concept, e.g. "modal state management" or "API error
handling"] is implemented across the codebase. Flag every place it deviates
from the dominant pattern, and state which deviations are intentional
(documented reason) vs. accidental drift.
```

---

## Dead code / unused export audit

**What it catches:** functions, components, CSS classes, and exports nothing
references anymore — common after refactors like the one you just did.

```
Find unused exports, unreferenced components, and CSS classes with no
matching usage in [directory]. For each, confirm it's actually unused (not
just hard to grep for — check dynamic imports/string references too) before
listing it as a removal candidate.
```

---

## Type-safety audit (TypeScript-specific)

**What it catches:** `any` creep, unsafe casts, places where the type system
isn't actually protecting you even though it looks like it is.

```
Scan for `any`, `as` casts, and `@ts-ignore`/`@ts-expect-error` in [directory].
For each, state whether it's masking a real type mismatch or is a legitimate
escape hatch, and whether a narrower type is achievable without much churn.
```

---

## Duplication audit

**What it catches:** near-identical logic copy-pasted with small variations —
different from the smell audit's "duplicated logic" in that this one
specifically hunts for copies, not just repeated patterns.

```
Find blocks of near-duplicate code (not just literally identical — similar
structure with minor variable differences) across [directory]. For each
cluster, suggest whether it's worth extracting into a shared function/
composable, or if the duplication is incidental and extraction would hurt
readability.
```

---

## Dependency audit

**What it catches:** unused packages inflating the bundle/build, outdated
packages with known issues, or dependencies doing a job the stdlib/framework
now covers natively.

```
Review package.json / Cargo.toml against actual usage. Flag: dependencies
with no remaining imports anywhere in the codebase, dependencies that could
be replaced by something already in use, and anything notably out of date.
```

---

## Security / input-handling audit

**What it catches:** the boring-but-real stuff — unvalidated input reaching
the DB or filesystem, auth checks that can be bypassed, secrets in code.

```
Review [endpoint/file] for input validation gaps, missing auth checks on
admin-only routes, SQL built via string concatenation instead of parameterized
queries, and any hardcoded secrets or credentials. State severity, not just
presence.
```

---

## Using more than one at once

The smell + correctness pairing is the one worth defaulting to together,
since a clean smell result on its own tells you nothing about whether the
code works — that's the gap that came up earlier today. For a full sweep
after a big feature lands, smell + correctness + architecture-consistency is
a solid trio without going overboard.
