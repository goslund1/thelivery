#!/usr/bin/env bash
# Rolls back to the previous release by repointing the 'current' symlink.
# Run on the droplet: sudo bash /opt/thelivery/rollback.sh
set -euo pipefail

RELEASES=/opt/thelivery/releases
SVC=thelivery

current=$(readlink "$RELEASES/current")
echo "Current release: $current"

# All release dirs sorted newest-first; skip the symlink itself.
mapfile -t all < <(find "$RELEASES" -maxdepth 1 -mindepth 1 -type d | sort -r)

if [ "${#all[@]}" -lt 2 ]; then
    echo "No previous release to roll back to." >&2
    exit 1
fi

# Find the entry after 'current' in the sorted list.
prev=""
found=false
for r in "${all[@]}"; do
    if $found; then prev="$r"; break; fi
    [ "$r" = "$current" ] && found=true
done

if [ -z "$prev" ]; then
    echo "Could not determine previous release." >&2
    exit 1
fi

echo "Rolling back to: $prev"
ln -sfn "$prev" "$RELEASES/current"
systemctl restart "$SVC"
sleep 1
systemctl --no-pager --full status "$SVC" | head -n 8
echo "==> Rollback complete"
