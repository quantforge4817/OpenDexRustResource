#!/bin/bash
#
# OpenDex Matrix Server Setup Script
# Automated setup for Conduit on a cloud VPS
#
# Usage: sudo bash setup-matrix.sh matrix.opendex.org
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run as root (sudo)${NC}"
    exit 1
fi

# Check for domain argument
if [ -z "${1:-}" ]; then
    echo -e "${RED}Error: Domain name required${NC}"
    echo "Usage: sudo bash setup-matrix.sh matrix.opendex.org"
    exit 1
fi

DOMAIN="$1"
SERVER_NAME="${DOMAIN#*.}"  # Extract server name (e.g., opendex.org from matrix.opendex.org)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN} OpenDex Matrix Server Setup${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Domain: $DOMAIN"
echo "Server name: $SERVER_NAME"
echo ""

# Generate a random registration token
REGISTRATION_TOKEN=$(openssl rand -hex 16)
echo -e "${YELLOW}Registration token: $REGISTRATION_TOKEN${NC}"
echo "Save this token — you'll need it for user registration!"
echo ""

read -p "Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# ========================================
# Step 1: Install Docker
# ========================================
echo -e "\n${GREEN}[1/6] Installing Docker...${NC}"

if command -v docker &> /dev/null; then
    echo "Docker already installed."
else
    apt-get update
    apt-get install -y ca-certificates curl gnupg lsb-release
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    chmod a+r /etc/apt/keyrings/docker.gpg

    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
      $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

    apt-get update
    apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
    systemctl enable docker
    systemctl start docker
fi

echo -e "${GREEN}✓ Docker installed${NC}"

# ========================================
# Step 2: Install Nginx and Certbot
# ========================================
echo -e "\n${GREEN}[2/6] Installing Nginx and Certbot...${NC}"

apt-get install -y nginx certbot python3-certbot-nginx

echo -e "${GREEN}✓ Nginx and Certbot installed${NC}"

# ========================================
# Step 3: Configure Nginx
# ========================================
echo -e "\n${GREEN}[3/6] Configuring Nginx...${NC}"

# Create Nginx config
cat > /etc/nginx/sites-available/matrix << NGINX_EOF
# Matrix server reverse proxy

# Client API (HTTPS)
server {
    listen 443 ssl http2;
    server_name $DOMAIN;

    # SSL certificates (managed by Certbot)
    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;

    # SSL settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache shared:SSL:10m;

    # Security headers
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;

    # Client API
    location /_matrix {
        proxy_pass http://127.0.0.1:6167;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;

        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";

        # Timeouts
        proxy_read_timeout 290s;
        proxy_send_timeout 290s;
    }

    # .well-known discovery
    location /.well-known/matrix/server {
        default_type application/json;
        return 200 '{"server_name": "$SERVER_NAME", "port": 8448}';
    }

    location /.well-known/matrix/client {
        default_type application/json;
        return 200 '{"m.homeserver": {"base_url": "https://$DOMAIN"}}';
    }
}

# Federation port (for Matrix federation traffic)
server {
    listen 8448 ssl http2;
    server_name $DOMAIN;

    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location /_matrix {
        proxy_pass http://127.0.0.1:6167;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}

# HTTP -> HTTPS redirect
server {
    listen 80;
    server_name $DOMAIN;

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }

    location / {
        return 301 https://\$host\$request_uri;
    }
}
NGINX_EOF

# Enable the site
ln -sf /etc/nginx/sites-available/matrix /etc/nginx/sites-enabled/matrix
rm -f /etc/nginx/sites-enabled/default  # Remove default site

# Create certbot challenge directory
mkdir -p /var/www/certbot

# Test Nginx config
nginx -t

echo -e "${GREEN}✓ Nginx configured${NC}"

# ========================================
# Step 4: Obtain HTTPS Certificate
# ========================================
echo -e "\n${GREEN}[4/6] Obtaining HTTPS certificate...${NC}"

# Create temporary Nginx config for certbot (before we have certs)
cat > /etc/nginx/sites-available/matrix-temp << TEMP_NGINX_EOF
server {
    listen 80;
    server_name $DOMAIN;

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }

    location / {
        return 301 https://\$host\$request_uri;
    }
}
TEMP_NGINX_EOF

ln -sf /etc/nginx/sites-available/matrix-temp /etc/nginx/sites-enabled/matrix-temp
rm -f /etc/nginx/sites-enabled/matrix
nginx -t && systemctl reload nginx

# Obtain certificate
certbot --nginx --non-interactive --agree-tos --register-unsafely-without-email -d "$DOMAIN" || {
    echo -e "${YELLOW}Warning: Certbot failed. You may need to obtain the certificate manually.${NC}"
    echo "Run: sudo certbot --nginx -d $DOMAIN"
}

# Restore the full Nginx config
rm -f /etc/nginx/sites-enabled/matrix-temp
ln -sf /etc/nginx/sites-available/matrix /etc/nginx/sites-enabled/matrix
nginx -t && systemctl reload nginx

echo -e "${GREEN}✓ HTTPS certificate obtained${NC}"

# ========================================
# Step 5: Configure Conduit
# ========================================
echo -e "\n${GREEN}[5/6] Configuring Conduit...${NC}"

# Generate Conduit config
cat > "$SCRIPT_DIR/conduit-generated.toml" << CONDUIT_EOF
# Conduit configuration for OpenDex Matrix server
# Generated by setup-matrix.sh

# Server name (the domain part after @ in user IDs)
server_name = "$SERVER_NAME"

# Registration
[registration]
# Require a token for registration (change to false for open registration)
require_token = true
# The registration token (keep this secure!)
token = "$REGISTRATION_TOKEN"
# Allow registration (set to false to disable new registrations)
enabled = true

# Federation
[federation]
# Enable federation with other Matrix servers
enabled = true
# Listen address for federation
listen_addresses = ["0.0.0.0:6167"]

# Trusted servers for federation (for initial room discovery)
trusted_servers = ["matrix.org"]

# Database
[database]
# Use RocksDB (default, best performance)
path = "/data/conduit.db"

# Logging
[logging]
level = "info"
CONDUIT_EOF

echo -e "${GREEN}✓ Conduit configured${NC}"

# ========================================
# Step 6: Start Conduit
# ========================================
echo -e "\n${GREEN}[6/6] Starting Conduit...${NC}"

cd "$SCRIPT_DIR"

# Copy the generated config
cp conduit-generated.toml conduit.toml

# Start Conduit
docker compose up -d

# Wait for Conduit to start
echo "Waiting for Conduit to start..."
sleep 5

# Check if Conduit is running
if docker compose ps | grep -q "Up"; then
    echo -e "${GREEN}✓ Conduit is running!${NC}"
else
    echo -e "${RED}✗ Conduit failed to start. Check logs:${NC}"
    docker compose logs
    exit 1
fi

# ========================================
# Firewall Configuration
# ========================================
echo -e "\n${GREEN}Configuring firewall...${NC}"

if command -v ufw &> /dev/null; then
    ufw allow OpenSSH
    ufw allow 'Nginx Full'
    ufw allow 8448/tcp  # Matrix federation
    echo "y" | ufw enable
    echo -e "${GREEN}✓ Firewall configured${NC}"
else
    echo -e "${YELLOW}UFW not found. Please configure your firewall manually.${NC}"
    echo "Allow ports: 22 (SSH), 80 (HTTP), 443 (HTTPS), 8448 (Matrix federation)"
fi

# ========================================
# Setup Complete
# ========================================
echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN} Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Your Matrix server is running at: https://$DOMAIN"
echo ""
echo "Next steps:"
echo "1. Create an admin user:"
echo "   curl -X POST http://localhost:6167/_matrix/client/r0/admin/register \\"
echo "     -u admin:your-password \\"
echo "     -d '{\"username\": \"admin\", \"password\": \"your-password\", \"admin\": true, \"displayname\": \"Admin\"}'"
echo ""
echo "2. Create the community room:"
echo "   See docs/matrix/README.md for room creation instructions"
echo ""
echo "3. Set up daily backups:"
echo "   sudo bash backup.sh"
echo "   sudo crontab -e"
echo "   # Add: 0 2 * * * /path/to/backup.sh"
echo ""
echo "4. Update DNS records:"
echo "   A record: $DOMAIN -> <your-VPS-IP>"
echo "   TXT record: _matrix._tcp.$DOMAIN -> matrix://$DOMAIN"
echo ""
echo -e "${YELLOW}Important: Save your registration token: $REGISTRATION_TOKEN${NC}"
echo ""
