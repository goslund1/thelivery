#!/usr/bin/env bash
# One-time TLS setup on the droplet. Installs Caddy, points it at the backend,
# and lets it auto-obtain + auto-renew a Let's Encrypt certificate.
#
# Prereqs:
#   - Your domain's DNS A record points at this droplet's IP.
#   - Ports 80 and 443 are open in the droplet / DO cloud firewall.
#   - The thelivery service is running on 127.0.0.1:8787 (deployed already).
#
# Usage (on the droplet):
#   sudo bash setup-caddy.sh your-domain.com
set -euo pipefail

DOMAIN="${1:?usage: setup-caddy.sh <domain>}"

echo "==> Installing Caddy (official apt repo)"
sudo apt-get update
sudo apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
  | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
  | sudo tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
sudo apt-get update
sudo apt-get install -y caddy

echo "==> Writing /etc/caddy/Caddyfile for $DOMAIN"
sudo tee /etc/caddy/Caddyfile >/dev/null <<EOF
$DOMAIN {
	encode zstd gzip
	reverse_proxy 127.0.0.1:8787
}
EOF

echo "==> Reloading Caddy (obtains the certificate on first run)"
sudo systemctl reload caddy || sudo systemctl restart caddy
sleep 2
sudo systemctl --no-pager status caddy | head -n 12
echo "==> Done. Visit https://$DOMAIN"
