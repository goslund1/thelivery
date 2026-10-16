#!/bin/bash
# order-gate: checks the two inboxes, Drop Offs/ and Cloud Drop Offs/,
# for what must not be committed, because a push can't be taken back
# (~/Projects/standards/STANDARD.md, "Cloud sessions"; D16).
#
#   tools/order-gate.sh             what is staged in the inboxes (the pre-commit hook runs this)
#   tools/order-gate.sh --all       every file in the inboxes, staged or not
#   tools/order-gate.sh <file>...   the files named: run it before putting an order in an inbox
#
# A STOP line makes it exit 1, and the commit doesn't happen. A note line
# is for a person to look at and doesn't stop anything.
# To let a STOP through for one commit, on Jason's say only:
#   ORDER_GATE=off git commit ...
# Version 1, 2026-10-09.
set -uo pipefail
case "${1:-}" in
  ""|--all) cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)" || exit 2 ;;
esac

python3 - "$@" <<'PY'
import os, re, subprocess, sys

INBOXES = ('Drop Offs', 'Cloud Drop Offs')
TEXT = ('.md', '.markdown', '.txt', '.tsv', '.csv', '.json', '.yml', '.yaml')
stops, notes = [], []

def octets_ok(m): return all(int(p) <= 255 for p in m.group(0).split('.')) and m.group(0) not in ('127.0.0.1', '0.0.0.0')
def email_ok(m):
    local, _, domain = m.group(0).partition('@')
    return local.lower() not in ('git', 'noreply') and not domain.lower().endswith(('example.com', 'noreply.github.com'))

STOP = [
  ('an email address', re.compile(r'[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}'), email_ok),
  ('a phone number', re.compile(r'(?<![\w.-])(?:\+?1[ .-]?)?\(?\d{3}\)?[ .-]\d{3}[ .-]\d{4}(?![\w-])'), None),
  ('a key or token', re.compile(r'\b(?:ghp_|gho_|ghs_|github_pat_)[A-Za-z0-9_]{20,}|\bsk-[A-Za-z0-9_-]{20,}|\bAKIA[0-9A-Z]{16}\b|\bxox[baprs]-[A-Za-z0-9-]{10,}'), None),
  ('a private key', re.compile(r'-----BEGIN [A-Z ]*PRIVATE KEY-----'), None),
]
NOTE = [
  ('a home-folder path', re.compile(r'/Users/[^/\s`"\']+/'), None),
  ('a picture or video file name (Jason\'s own pictures are never named in a doc)',
   re.compile(r'[\w()\-]+\.(?:jpe?g|heic|heif|png|webp|gif|tiff?|dng|mov|mp4|m4v)\b', re.I), None),
  ('a network address', re.compile(r'(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w.])'), octets_ok),
  ('what may be a street address', re.compile(r'\b\d{2,6}\s+(?:[NSEW]\.?\s+)?[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?\s+(?:St|Street|Ave|Avenue|Rd|Road|Blvd|Boulevard|Dr|Drive|Ln|Lane|Ct|Court|Pl|Place)\b'), None),
  ('a password or secret written out', re.compile(r'(?i)\b(?:password|passwd|passphrase|secret|api[_-]?key)\b\s*[:=]\s*\S{6,}'), None),
]

def mask(s):
    s = s.strip()
    return s if len(s) <= 6 else s[:4] + '...' + s[-2:]

def scan(name, data):
    if os.path.basename(name) == '.DS_Store': return
    text = None
    if name.lower().endswith(TEXT) and b'\0' not in data:
        try: text = data.decode('utf-8')
        except UnicodeDecodeError: pass
    if text is None:
        stops.append(f'{name}: not a text file. Pictures and other files stay out of an inbox that is committed')
        return
    for n, line in enumerate(text.splitlines(), 1):
        for what, rx, ok in STOP:
            for m in rx.finditer(line):
                if ok is None or ok(m): stops.append(f'{name}:{n}: {what} ({mask(m.group(0))})')
        for what, rx, ok in NOTE:
            for m in rx.finditer(line):
                if ok is None or ok(m): notes.append(f'{name}:{n}: {what} ({mask(m.group(0))})')

args = sys.argv[1:]
if args and args[0] != '--all':
    for f in args:
        if os.path.isdir(f):
            for d, _, fs in os.walk(f):
                for x in sorted(fs): scan(os.path.join(d, x), open(os.path.join(d, x), 'rb').read())
        elif os.path.exists(f): scan(f, open(f, 'rb').read())
        else: stops.append(f'{f}: no such file')
elif args:
    for box in INBOXES:
        for d, _, fs in os.walk(box):
            for x in sorted(fs): scan(os.path.join(d, x), open(os.path.join(d, x), 'rb').read())
else:
    out = subprocess.run(['git', 'diff', '--cached', '--name-only', '-z', '--diff-filter=ACMR', '--', *INBOXES],
                         capture_output=True).stdout
    for f in [x for x in out.decode('utf-8', 'replace').split('\0') if x]:
        scan(f, subprocess.run(['git', 'show', ':' + f], capture_output=True).stdout)

for m in stops: print('order-gate: STOP ' + m)
for m in notes: print('order-gate: note ' + m)
if stops and os.environ.get('ORDER_GATE') == 'off':
    print(f'order-gate: {len(stops)} stop(s) let through by ORDER_GATE=off'); sys.exit(0)
if stops:
    print(f'order-gate: {len(stops)} stop(s), {len(notes)} note(s). Nothing was committed.'); sys.exit(1)
if notes: print(f'order-gate: ok ({len(notes)} note(s) to look at)')
PY
