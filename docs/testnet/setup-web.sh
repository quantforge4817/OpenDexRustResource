#!/usr/bin/env bash
# setup-web.sh — Build and deploy the OpenDex web frontend
#
# Builds the Leptos/WASM static site and configures Nginx with HTTPS.
# Run as root on the VPS after running setup-vps.sh.
#
# Usage:
#   sudo bash setup-web.sh [domain] [repo-path]
#
# Arguments:
#   domain    — Domain name for HTTPS (e.g., testnet.opendex.io). Default: localhost
#   repo-path — Path to the OpenDex repository. Default: /home/opendex/opendex

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# --- Arguments ---
DOMAIN="${1:-localhost}"
REPO_PATH="${2:-/home/opendex/opendex}"

info "=== OpenDex Web Frontend Deployment ==="
info "Domain: $DOMAIN"
info "Repository: $REPO_PATH"

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (sudo)."
    exit 1
fi

# Check for repo
if [[ ! -d "$REPO_PATH" ]]; then
    error "Repository not found at $REPO_PATH"
    info "Clone the repo first: sudo -u opendex git clone <repo-url> $REPO_PATH"
    exit 1
fi

FRONTEND_DIR="$REPO_PATH/crates/web-frontend"
if [[ ! -d "$FRONTEND_DIR" ]]; then
    error "Web frontend directory not found: $FRONTEND_DIR"
    exit 1
fi

# --- Check for Rust & Trunk ---
if ! command -v cargo &>/dev/null; then
    info "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

if ! command -v trunk &>/dev/null; then
    info "Trunk not found. Installing..."
    cargo install trunk
fi

if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    info "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# --- Build the static site ---
info "Building web frontend (release mode)..."
cd "$FRONTEND_DIR"
sudo -u opendex trunk build --release

DIST_DIR="$FRONTEND_DIR/dist"
if [[ ! -d "$DIST_DIR" ]]; then
    error "Build failed — dist/ directory not found."
    exit 1
fi

FILE_COUNT=$(find "$DIST_DIR" -type f | wc -l)
TOTAL_SIZE=$(du -sh "$DIST_DIR" | cut -f1)
info "Build complete: $FILE_COUNT files, $TOTAL_SIZE"

# --- Deploy to web root ---
WEB_ROOT="/var/www/opendex"
info "Deploying to $WEB_ROOT..."
mkdir -p "$WEB_ROOT"
rm -rf "$WEB_ROOT"/*
cp -r "$DIST_DIR"/* "$WEB_ROOT"/
chown -R www-data:www-data "$WEB_ROOT"
chmod -R 755 "$WEB_ROOT"
info "Files deployed."

# --- Configure Nginx ---
info "Configuring Nginx..."

# Create Nginx config with domain
cat > /etc/nginx/sites-available/opendex <<NGINX_EOF
# OpenDex Web Portal — Nginx Configuration
server {
    listen 80;
    server_name ${DOMAIN};

    root ${WEB_ROOT};
    index index.html;

    # Security headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';" always;
    add_header Permissions-Policy "geolocation=(), microphone=(), camera=()" always;

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml application/xml+rss text/javascript image/svg+xml;
    gzip_min_length 1000;

    # Serve static assets with long cache
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|wasm)\$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # SPA routing: all routes fall back to index.html
    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # Block access to hidden files
    location ~ /\. {
        deny all;
    }

    # Custom error pages
    error_page 404 /index.html;
}
NGINX_EOF

# Enable site
rm -f /etc/nginx/sites-enabled/default
ln -sf /etc/nginx/sites-available/opendex /etc/nginx/sites-enabled/opendex

# Test and reload
nginx -t
systemctl reload nginx
info "Nginx configured and reloaded."

# --- HTTPS via Certbot ---
if [[ "$DOMAIN" != "localhost" ]]; then
    info "Setting up HTTPS via Let's Encrypt..."
    if command -v certbot &>/dev/null; then
        certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos --email admin@"$DOMAIN" || {
            warn "Certbot failed. You can run it manually later:"
            warn "  certbot --nginx -d $DOMAIN"
        }
    else
        warn "Certbot not installed. To enable HTTPS later:"
        warn "  apt-get install certbot python3-certbot-nginx"
        warn "  certbot --nginx -d $DOMAIN"
    fi
else
    info "Skipping HTTPS for localhost. To enable HTTPS with a domain:"
    info "  certbot --nginx -d $DOMAIN"
fi

# --- Summary ---
info ""
info "=== Web Frontend Deployment Complete ==="
info "Domain: $DOMAIN"
info "Web root: $WEB_ROOT"
info ""
info "Access the site at:"
if [[ "$DOMAIN" == "localhost" ]]; then
    info "  http://localhost"
else
    info "  http://$DOMAIN"
    info "  https://$DOMAIN (if HTTPS was configured)"
fi
info ""
info "Pages:"
info "  /        — Home page"
info "  /source  — Source browser"
info "  /legal   — Legal information"
info ""
info "Check Nginx status:"
info "  systemctl status nginx"
info "  journalctl -u nginx -f"
