#!/bin/bash
# docs-check: checks a project's docs against the Projects standard
# (~/Projects/standards/STANDARD.md). Run it at every clear:
#
#   tools/docs-check.sh            from anywhere inside the project
#   tools/docs-check.sh <folder>   to check another project
#
# It prints each problem. FAIL lines make it exit 1; warn lines don't.
# Version 1, 2026-10-09, generalised from RhythmIO's tools/docs-check.sh.
#
# Optional settings in spec/docs-check.conf, one KEY=value per line:
#   BACKLOG_PREFIX=B     the letters before the dash in backlog IDs (B-12)
#   SPEC_WARN_KB=40      warn when a live spec is bigger than this
set -uo pipefail
if [ $# -ge 1 ]; then cd "$1" || exit 2; else cd "$(dirname "$0")/.." || exit 2; fi

python3 - <<'PY'
import glob, os, re, subprocess, sys

fails, warns = [], []
def fail(m): fails.append(m)
def warn(m): warns.append(m)
def read(f):
    with open(f, encoding='utf-8', errors='replace') as h: return h.read()
def size(f): return os.path.getsize(f)

conf = {'BACKLOG_PREFIX': 'B', 'SPEC_WARN_KB': '40'}
if os.path.exists('spec/docs-check.conf'):
    for line in read('spec/docs-check.conf').splitlines():
        if '=' in line and not line.strip().startswith('#'):
            k, v = line.split('=', 1); conf[k.strip()] = v.strip()
P = re.escape(conf['BACKLOG_PREFIX'])

def live_specs():
    out = []
    for f in sorted(glob.glob('spec/*.md') + glob.glob('spec/*/*.md')):
        if f.startswith('spec/history/'): continue
        out.append(f)
    return out
specs = live_specs()
skills = sorted(glob.glob('.claude/skills/*/SKILL.md'))
rules = sorted(glob.glob('.claude/rules/**/*.md', recursive=True))

# 1. CLAUDE.md: there, short, and holding no state.
if not os.path.exists('CLAUDE.md'):
    fail('no CLAUDE.md at the project root')
    claude = ''
else:
    claude = read('CLAUDE.md')
    n = claude.count('\n')
    if size('CLAUDE.md') > 20_000: fail(f'CLAUDE.md is {size("CLAUDE.md"):,} bytes (cap 20,000)')
    if n > 200: warn(f'CLAUDE.md is {n} lines (aim for 200 or fewer; move code traps to a skill or .claude/rules/)')
    if re.search(r'^#+ *(Current state|Status|Where it stands)\b', claude, re.M | re.I):
        warn('CLAUDE.md has a state section; state belongs in spec/status.md')

# 2. status.md: there, short, present tense.
if not os.path.exists('spec/status.md'):
    fail('no spec/status.md (the file every session reads first)')
else:
    t = read('spec/status.md'); n = t.count('\n')
    if n > 200: fail(f'spec/status.md is {n} lines (rule: 150)')
    elif n > 150: warn(f'spec/status.md is {n} lines (rule: 150)')
    if size('spec/status.md') > 10_000: fail(f'spec/status.md is {size("spec/status.md"):,} bytes (cap 10,000)')
    dated = len(re.findall(r'^\s*([-*0-9.]+\s+)?(\*\*)?[^|`\n]{0,40}\b20\d\d-\d\d-\d\d\b', t, re.M))
    if dated > 15: fail(f'spec/status.md has {dated} lines starting with a date; move the stories to spec/history/')

# 3. Every spec/ path a live doc names exists.
for doc in (['CLAUDE.md'] if claude else []) + specs + skills + rules:
    for path in sorted(set(re.findall(r'`(spec/[A-Za-z0-9._/*-]+)`', read(doc)))):
        path = path.rstrip('.,:;')
        if '*' in path:
            if not glob.glob(path): fail(f'{doc} names {path}, which matches nothing')
        elif not os.path.exists(path):
            fail(f'{doc} names {path}, which does not exist')

# 4. CLAUDE.md's docs table names every live spec.
for f in specs:
    if f == 'spec/docs-check.conf': continue
    if f'`{f}`' not in claude: fail(f'{f} is not named in CLAUDE.md (add it to the docs table)')

# 5. History has an index, and the index names every file.
if os.path.isdir('spec/history'):
    if not os.path.exists('spec/history/README.md'):
        fail('spec/history/ has no README.md index')
    else:
        idx = read('spec/history/README.md')
        for f in sorted(glob.glob('spec/history/*.md')):
            name = os.path.basename(f)
            if name != 'README.md' and name not in idx:
                fail(f'spec/history/README.md has no row for {name}')

# 6. Backlogs: open items only, short, one ID series.
backlogs = [b for b in ['spec/backlog.md'] + sorted(glob.glob('spec/*/backlog.md')) if os.path.exists(b)]
entry = re.compile(rf'^- \*\*({P}-(\d+))\*\*.*$', re.M)
seen = {}
nxt = None
if os.path.exists('spec/backlog.md'):
    m = re.search(rf'Next free ID: {P}-(\d+)', read('spec/backlog.md'))
    nxt = int(m.group(1)) if m else None
for b in backlogs:
    t = read(b)
    if size(b) > 45_000: fail(f'{b} is {size(b):,} bytes (cap 45,000)')
    for m in entry.finditer(t):
        line, bid, num = m.group(0), m.group(1), int(m.group(2))
        if re.search(r'\*\*(Built|built|Fixed|fixed|Done|done|Found and fixed)', line) or 'waiting on Jason' in line or '~~' in line:
            fail(f'{b}: {bid} is finished; close it (story to history, row to the checklist)')
        if len(line) > 600: fail(f'{b}: {bid} is {len(line)} characters (600 at most)')
        if bid in seen: fail(f'{bid} is an entry in both {seen[bid]} and {b}')
        seen[bid] = b
        if nxt is not None and num >= nxt: fail(f'{b}: {bid} is at or above the next free ID, {conf["BACKLOG_PREFIX"]}-{nxt}')
if seen and nxt is None:
    fail(f'spec/backlog.md has no "Next free ID: {conf["BACKLOG_PREFIX"]}-NN" line')

# 7. Checklists: one short row per feature, IDs never reused.
row = re.compile(r'^- \[[ xX]\] (.*)$', re.M)
ids = {}
for c in [c for c in ['spec/shakedown.md'] + sorted(glob.glob('spec/*/shakedown.md')) if os.path.exists(c)]:
    if size(c) > 45_000: fail(f'{c} is {size(c):,} bytes (cap 45,000)')
    for m in row.finditer(read(c)):
        line = m.group(0)
        if len(line) > 450: fail(f'{c}: a row is {len(line)} characters (450 at most): {line[:60]}')
        idm = re.match(r'\*\*([A-Z]+-\d+) · ', m.group(1))
        if idm:
            if idm.group(1) in ids: fail(f'{idm.group(1)} is used twice ({ids[idm.group(1)]}, {c})')
            ids[idm.group(1)] = c

# 8. Specs: open work that exists, and a size warning.
special = {'status.md', 'backlog.md', 'shakedown.md', 'README.md'}
warn_bytes = int(conf['SPEC_WARN_KB']) * 1024
for f in specs:
    base = os.path.basename(f)
    if base in special: continue
    t = read(f)
    if size(f) > warn_bytes:
        warn(f'{f} is {size(f) // 1024} KB (over {conf["SPEC_WARN_KB"]}): if it is Built, move its play-by-play to history')
    for m in re.finditer(r'Open work', t):
        i, depth, j = m.end(), 0, m.end()
        while j < len(t):
            ch = t[j]
            if ch in '([': depth += 1
            elif ch in ')]': depth -= 1
            elif ch == '.' and depth <= 0 and (j + 1 == len(t) or t[j + 1] in ' \n'): break
            elif t[j:j + 2] == '\n\n': break
            j += 1
        for bid in dict.fromkeys(re.findall(rf'{P}-\d+', t[i:j])):
            if seen and bid not in seen: fail(f'{f}: its Open work names {bid}, which is in no backlog')

# 9. No version numbers in live file names.
for f in specs + sorted(glob.glob('*.md')):
    if re.search(r'_\d+_\d+\.md$', f): warn(f'{f} has a version number in its name (git holds the versions)')

# 10. The inbox is never committed.
try:
    tracked = subprocess.run(['git', 'ls-files', 'Drop Offs'], capture_output=True, text=True).stdout.strip()
    if tracked: warn('files in Drop Offs/ are tracked by git; add "Drop Offs/" to .gitignore')
except Exception:
    pass

# 11. Nothing settles at the repo root.
for f in sorted(glob.glob('*.md')):
    if f not in ('CLAUDE.md', 'CLAUDE.local.md', 'README.md', 'LICENSE.md'):
        warn(f'{f} sits at the repo root; file it under spec/ or spec/history/')

for m in fails: print('docs-check: FAIL ' + m)
for m in warns: print('docs-check: warn ' + m)
if fails:
    print(f'docs-check: {len(fails)} failure(s), {len(warns)} warning(s)'); sys.exit(1)
print('docs-check: ok' + (f' ({len(warns)} warning(s))' if warns else ''))
PY
