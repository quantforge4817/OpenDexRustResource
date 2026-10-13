#!/bin/bash
# Deploy OpenDex Web Portal to a VPS
#
# Usage (from project root):
#   cd docs/deployment
#   ./deploy.sh <user>@<host>
#   ./deploy.sh deploy@your-server.com
#
# This script:
# 1. Builds the site locally
# 2. Uploads the dist/ folder to the VPS via SCP
# 3. Sets up Nginx configuration
# 4. Configures SSL with Let's Encrypt (optional)
#
# Prerequisites:
#   - SSH access to your VPS
#   - sudo access on the VPS
#   - Domain name pointing to your VPS IP (for SSL)

set -euo pipefail

if [[ -z "${1:-}" ]]; then
    echo "Usage: $0 <user>@<host>"
    echo "Example: $0 deploy@your-server.com"
    exit 1
fi

REMOTE_USER_HOST="$1"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
REMOTE_DIR="/var/www/opendex"
NGINX_CONF="$SCRIPT_DIR/deploy/nginx.conf"

echo "============================================"
echo "  OpenDex Web Portal Deployment"
echo "  Target: $REMOTE_USER_HOST"
echo "============================================"

# Step 1: Build the site
echo ""
echo "Step 1: Building the site..."
bash "$SCRIPT_DIR/build.sh" --release

if [[ ! -d "$DIST_DIR" ]]; then
    echo "ERROR: Build failed. dist/ directory not found."
    exit 1
fi

# Step 2: Upload to VPS
echo ""
echo "Step 2: Uploading to VPS..."
echo "Creating remote directory..."
ssh "$REMOTE_USER_HOST" "sudo mkdir -p $REMOTE_DIR"

echo "Uploading files..."
rsync -avz --delete "$DIST_DIR/" "$REMOTE_USER_HOST:$REMOTE_DIR/"

echo "Setting permissions..."
ssh "$REMOTE_USER_HOST" "sudo chown -R www-data:www-data $REMOTE_DIR"
ssh "$REMOTE_USER_HOST" "sudo chmod -R 755 $REMOTE_DIR"

# Step 3: Install Nginx config
echo ""
echo "Step 3: Installing Nginx configuration..."
echo "Uploading Nginx config..."
scp "$NGINX_CONF" "$REMOTE_USER_HOST:/tmp/opendex-nginx.conf"

echo "Installing Nginx config..."
ssh "$REMOTE_USER_HOST" "sudo mv /tmp/opendex-nginx.conf /etc/nginx/sites-available/opendex"
ssh "$REMOTE_USER_HOST" "sudo ln -sf /etc/nginx/sites-available/opendex /etc/nginx/sites-enabled/opendex"
ssh "$REMOTE_USER_HOST" "sudo nginx -t && sudo systemctl reload nginx"

echo ""
echo "============================================"
echo "  Deployment complete!"
echo "============================================"
echo ""
echo "Your site should now be accessible at:"
echo "  http://<your-domain>"
echo ""
echo "For HTTPS with Let's Encrypt, run:"
echo "  sudo certbot --nginx -d your-domain.com"
echo ""
echo "To update the site in the future, simply run:"
echo "  ./deploy.sh $REMOTE_USER_HOST"
