#!/usr/bin/env bash
# Runs ON the droplet (piped in over SSH by the GitHub Action). Idempotent:
# creates the service user/dirs on first run, then swaps in the new binary,
# static frontend, seed data, and systemd unit, and restarts the service.
# Requires passwordless sudo for the SSH (deploy) user.
set -euo pipefail

APP_DIR=/opt/thelivery
SVC=thelivery
STAGE="$HOME/thelivery-stage"   # where the Action rsync'd the build bundle

echo "==> Ensuring service user and directories"
id -u "$SVC" >/dev/null 2>&1 || sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SVC"
sudo mkdir -p "$APP_DIR/static" "$APP_DIR/seed" "$APP_DIR/uploads"

echo "==> Installing binary"
sudo install -m 0755 "$STAGE/livery-backend" "$APP_DIR/livery-backend.new"
sudo mv -f "$APP_DIR/livery-backend.new" "$APP_DIR/livery-backend"

echo "==> Updating static frontend"
sudo rsync -a --delete "$STAGE/static/" "$APP_DIR/static/"

echo "==> Updating seed data"
sudo install -m 0644 "$STAGE/seed/cards.json" "$APP_DIR/seed/cards.json"

# Populate seed images on first deploy; never clobber images uploaded via the app.
echo "==> Seeding images (no-clobber)"
sudo cp -rn "$STAGE/seed-uploads/." "$APP_DIR/uploads/" 2>/dev/null || true

echo "==> Installing systemd unit"
sudo install -m 0644 "$STAGE/thelivery.service" /etc/systemd/system/thelivery.service

echo "==> Fixing ownership"
sudo chown -R "$SVC:$SVC" "$APP_DIR"

echo "==> Reloading and restarting service"
sudo systemctl daemon-reload
sudo systemctl enable "$SVC" >/dev/null 2>&1 || true
sudo systemctl restart "$SVC"
sleep 1
sudo systemctl --no-pager --full status "$SVC" | head -n 12
echo "==> Done"
