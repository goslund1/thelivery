#!/usr/bin/env bash
# Runs ON the droplet (piped in over SSH by the GitHub Action). Idempotent:
# creates the service user/dirs on first run, then installs the new release
# into a timestamped directory, repoints the 'current' symlink, and restarts.
# Permanent data (database, uploads, seed) lives outside the releases tree.
# Keeps the last 5 releases; older ones are pruned automatically.
set -euo pipefail

APP_DIR=/opt/thelivery
RELEASES="$APP_DIR/releases"
RELEASE="$RELEASES/$(date +%Y%m%d-%H%M%S)"
SVC=thelivery
STAGE="$HOME/thelivery-stage"
KEEP=5

echo "==> Ensuring service user and directories"
id -u "$SVC" >/dev/null 2>&1 || sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SVC"
sudo mkdir -p "$APP_DIR/uploads" "$APP_DIR/seed" "$RELEASES"

echo "==> Creating release: $RELEASE"
sudo mkdir -p "$RELEASE/static"

echo "==> Installing binary"
sudo install -m 0755 "$STAGE/livery-backend" "$RELEASE/livery-backend"

echo "==> Installing frontend"
sudo rsync -a --delete "$STAGE/static/" "$RELEASE/static/"

echo "==> Updating seed data"
sudo install -m 0644 "$STAGE/seed/cards.json" "$APP_DIR/seed/cards.json"
sudo install -m 0644 "$STAGE/seed/cars.json" "$APP_DIR/seed/cars.json"
sudo install -m 0644 "$STAGE/seed/liveries.json" "$APP_DIR/seed/liveries.json"
if [ -f "$STAGE/seed/users.json" ]; then
  sudo install -m 0600 "$STAGE/seed/users.json" "$APP_DIR/seed/users.json"
fi

# ONE-TIME (2026-07-17): retire the proof-of-concept database so the service
# re-seeds from the freshly shipped seed files on restart. The old DB is kept
# as a backup, and the guard file makes this a no-op on every later deploy.
# Revert this block in the next commit.
if [ ! -f "$APP_DIR/data.db.bak-20260717" ] && [ -f "$APP_DIR/data.db" ]; then
  echo "==> ONE-TIME: backing up and retiring old data.db for reseed"
  sudo mv "$APP_DIR/data.db" "$APP_DIR/data.db.bak-20260717"
  sudo rm -f "$APP_DIR/data.db-wal" "$APP_DIR/data.db-shm"
fi

echo "==> Seeding images (no-clobber)"
sudo cp -rn "$STAGE/seed-uploads/." "$APP_DIR/uploads/" 2>/dev/null || true

echo "==> Installing systemd unit"
sudo install -m 0644 "$STAGE/thelivery.service" /etc/systemd/system/thelivery.service

echo "==> Pointing 'current' symlink to new release"
sudo ln -sfn "$RELEASE" "$RELEASES/current"

echo "==> Installing rollback script"
sudo install -m 0755 "$STAGE/rollback.sh" "$APP_DIR/rollback.sh"

echo "==> Fixing ownership"
sudo chown -R "$SVC:$SVC" "$APP_DIR"

echo "==> Reloading and restarting service"
sudo systemctl daemon-reload
sudo systemctl enable "$SVC" >/dev/null 2>&1 || true
sudo systemctl restart "$SVC"
sleep 1
sudo systemctl --no-pager --full status "$SVC" | head -n 12

echo "==> Pruning old releases (keeping $KEEP)"
sudo bash -c "
  find '$RELEASES' -maxdepth 1 -mindepth 1 -type d | sort | head -n -$KEEP | xargs -r rm -rf
"

echo "==> Done — $(sudo find '$RELEASES' -maxdepth 1 -mindepth 1 -type d | wc -l) release(s) on disk"
