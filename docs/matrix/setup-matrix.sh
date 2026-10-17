#!/usr/bin/env bash
#
# setup-matrix.sh — Automated Conduit Matrix homeserver setup
#
# Usage: sudo bash setup-matrix.sh <domain>
# Example: sudo bash setup-matrix.sh matrix.opendex.org
#
# This script:
# 1. Installs Conduit from the latest GitHub release
# 2. Creates a systemd service for auto-start
# 3. Generates the Conduit configuration file
# 4. Configures Nginx as a reverse proxy
# 5. Sets up HTTPS via Let's Encrypt (Certbot)
# 6. Configures UFW firewall rules
#

set -euo pipefail

# --- Configuration ---
DOMAIN="${1:?Usage: sudo bash setup-matrix.sh <domain>}"
CONDUIT_USER="conduit"
CONDUIT_DATA_DIR="/var/lib/conduit"
CONDUIT_CONFIG_DIR="/etc/conduit"
CONDUIT_CONFIG_FILE="${CONDUIT_CONFIG_DIR}/conduit.toml"
CONDUIT_BIN="/usr/local/bin/conduit"
NGINX_SITE="/etc/nginx/sites-available/matrix"
NGINX_ENABLED="/etc/nginx/sites-enabled/matrix"

echo "============================================"
echo " OpenDex Matrix Server Setup"
echo "============================================"
echo "Domain: ${DOMAIN}"
echo "--------------------------------------------"

# --- Check root ---
if [[ $EUID -ne 0 ]]; then
  echo "ERROR: This script must be run as root (sudo)."
  exit 1
fi

# --- Check OS ---
if ! command -v lsb_release &> /dev/null && [[ ! -f /etc/os-release ]]; then
  echo "ERROR: Cannot detect OS. Ubuntu 24.04 LTS is recommended."
  exit 1
fi

if [[ -f /etc/os-release ]]; then
  . /etc/os-release
  echo "Detected OS: ${PRETTY_NAME}"
  if [[ "$ID" != "ubuntu" ]] && [[ "$ID" != "debian" ]]; then
    echo "WARNING: This script is tested on Ubuntu/Debian. Other distros may require manual steps."
  fi
fi

# --- Step 1: Install dependencies ---
echo ""
echo "[1/6] Installing dependencies..."
apt-get update
apt-get install -y wget curl tar nginx certbot python3-certbot-nginx ufw

# --- Step 2: Install Conduit ---
echo ""
echo "[2/6] Installing Conduit..."

# Download the latest release
CONDUIT_VERSION=$(curl -s https://api.github.com/repos/conduit-rms/conduit/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
echo "Downloading Conduit version: ${CONDUIT_VERSION}"

wget -q "https://github.com/conduit-rms/conduit/releases/download/${CONDUIT_VERSION}/conduit-x86_64-unknown-linux-musl.tar.gz" \
  -O /tmp/conduit.tar.gz

tar xzf /tmp/conduit.tar.gz -C /tmp/
mv /tmp/conduit "${CONDUIT_BIN}"
chmod +x "${CONDUIT_BIN}"
rm -f /tmp/conduit.tar.gz

echo "Conduit installed at ${CONDUIT_BIN}"
"${CONDUIT_BIN}" --version || true

# --- Step 3: Create user and directories ---
echo ""
echo "[3/6] Creating user and directories..."

if ! id "${CONDUIT_USER}" &>/dev/null; then
  useradd -r -s /bin/false -m -d "${CONDUIT_DATA_DIR}" "${CONDUIT_USER}"
  echo "Created user: ${CONDUIT_USER}"
else
  echo "User ${CONDUIT_USER} already exists."
fi

mkdir -p "${CONDUIT_DATA_DIR}"
mkdir -p "${CONDUIT_CONFIG_DIR}"
chown -R "${CONDUIT_USER}:${CONDUIT_USER}" "${CONDUIT_DATA_DIR}"
chown -R "${CONDUIT_USER}:${CONDUIT_USER}" "${CONDUIT_CONFIG_DIR}"

# --- Step 4: Generate configuration ---
echo ""
echo "[4/6] Generating Conduit configuration..."

# Generate a random registration shared secret
REGISTRATION_SECRET=$(openssl rand -hex 32)

# Find the script directory to locate conduit.toml template
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE="${SCRIPT_DIR}/conduit.toml"

if [[ -f "${TEMPLATE}" ]]; then
  # Use the template from the repo, replacing placeholders
  sed "s|{{DOMAIN}}|${DOMAIN}|g; s|{{REGISTRATION_SECRET}}|${REGISTRATION_SECRET}|g" \
    "${TEMPLATE}" > "${CONDUIT_CONFIG_FILE}"
  echo "Configuration generated from template."
else
  # Generate a minimal config if template is not found
  cat > "${CONDUIT_CONFIG_FILE}" <<EOF
# Conduit configuration for ${DOMAIN}
# Generated on $(date)

server_name = "${DOMAIN}"

[database]
url = "rocksdb://${CONDUIT_DATA_DIR}/db"

[registration]
shared_secret = "${REGISTRATION_SECRET}"

[federation]
enabled = true

[auto_updates]
enabled = true
EOF
  echo "Configuration generated (minimal, no template found)."
fi

chown "${CONDUIT_USER}:${CONDUIT_USER}" "${CONDUIT_CONFIG_FILE}"

echo "Registration shared secret: ${REGISTRATION_SECRET}"
echo "⚠️  Save this secret — you'll need it to register new users!"

# --- Step 5: Create systemd service ---
echo ""
echo "[5/6] Creating systemd service..."

cat > /etc/systemd/system/conduit.service <<EOF
[Unit]
Description=Conduit Matrix Homeserver
After=network.target

[Service]
Type=simple
User=${CONDUIT_USER}
Group=${CONDUIT_USER}
ExecStart=${CONDUIT_BIN} --config ${CONDUIT_CONFIG_FILE}
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${CONDUIT_DATA_DIR}
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable conduit
systemctl start conduit

if systemctl is-active --quiet conduit; then
  echo "Conduit service started successfully."
else
  echo "WARNING: Conduit service may have failed to start. Check logs:"
  echo "  journalctl -u conduit -n 50"
fi

# --- Step 6: Configure Nginx ---
echo ""
echo "[6/6] Configuring Nginx reverse proxy..."

# Find the nginx template
NGINX_TEMPLATE="${SCRIPT_DIR}/nginx-matrix.conf"

if [[ -f "${NGINX_TEMPLATE}" ]]; then
  sed "s|{{DOMAIN}}|${DOMAIN}|g" "${NGINX_TEMPLATE}" > "${NGINX_SITE}"
  echo "Nginx config generated from template."
else
  # Generate minimal Nginx config
  cat > "${NGINX_SITE}" <<EOF
# Matrix federation traffic (port 8448)
server {
    listen 8448 ssl;
    server_name ${DOMAIN};

    ssl_certificate /etc/letsencrypt/live/${DOMAIN}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${DOMAIN}/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:6167;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}

# Client-server API (port 443)
server {
    listen 443 ssl;
    server_name ${DOMAIN};

    ssl_certificate /etc/letsencrypt/live/${DOMAIN}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${DOMAIN}/privkey.pem;

    location /_matrix {
        proxy_pass http://127.0.0.1:6167;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_read_timeout 29030s;
        proxy_buffering off;
        chunked_transfer_encoding on;
        proxy_cache off;
    }

    location /.well-known/matrix/server {
        proxy_pass http://127.0.0.1:6167;
        proxy_set_header Host \$host;
    }

    location /.well-known/matrix/client {
        proxy_pass http://127.0.0.1:6167;
        proxy_set_header Host \$host;
    }
}

# HTTP -> HTTPS redirect
server {
    listen 80;
    server_name ${DOMAIN};
    return 301 https://\$host\$request_uri;
}
EOF
  echo "Nginx config generated (minimal, no template found)."
fi

# Enable the site
ln -sf "${NGINX_SITE}" "${NGINX_ENABLED}"
rm -f /etc/nginx/sites-enabled/default

# Test Nginx config
if nginx -t &>/dev/null; then
  echo "Nginx config test passed."
  systemctl reload nginx || systemctl start nginx
else
  echo "ERROR: Nginx config test failed. Check ${NGINX_SITE}"
  nginx -t
  exit 1
fi

# --- HTTPS setup ---
echo ""
echo "Setting up HTTPS via Let's Encrypt..."
echo "This requires ${DOMAIN} to point to this server's IP address."
echo ""

# Obtain certificate
certbot --nginx -d "${DOMAIN}" --non-interactive --register-unsafely-without-email --agree-tls --redirect || {
  echo ""
  echo "WARNING: Certbot failed. This is expected if DNS is not yet pointing to this server."
  echo "You can run Certbot manually later:"
  echo "  sudo certbot --nginx -d ${DOMAIN}"
}

# --- Firewall ---
echo ""
echo "Configuring UFW firewall..."

ufw allow OpenSSH
ufw allow 'Nginx Full'
ufw allow 8448/tcp  # Matrix federation

# Enable UFW non-interactively
echo "y" | ufw enable || true

echo ""
echo "============================================"
echo " Setup Complete!"
echo "============================================"
echo ""
echo "Domain: ${DOMAIN}"
echo "Config: ${CONDUIT_CONFIG_FILE}"
echo "Data:   ${CONDUIT_DATA_DIR}"
echo ""
echo "Next steps:"
echo "1. Register your admin user:"
echo "   conduit register --config ${CONDUIT_CONFIG_FILE} --username admin --password <password> --admin"
echo ""
echo "2. Create the OpenDex community room using a Matrix client (e.g., Element)"
echo ""
echo "3. Set up daily backups:"
echo "   chmod +x ${SCRIPT_DIR}/backup.sh"
echo "   sudo crontab -e"
echo "   # Add: 0 2 * * * ${SCRIPT_DIR}/backup.sh"
echo ""
echo "4. Monitor the service:"
echo "   sudo systemctl status conduit"
echo "   sudo journalctl -u conduit -f"
echo ""
echo "============================================"
