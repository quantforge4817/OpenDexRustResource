# OpenDex Matrix Server — Deployment Guide

This guide walks you through deploying a self-hosted **Conduit** Matrix homeserver on a cloud VPS for the OpenDex community chat.

## Overview

Conduit is a lightweight Matrix homeserver written in Rust. It runs as a single binary with an embedded RocksDB database, requiring minimal resources (~1 GB RAM). This guide deploys Conduit on a cheap cloud VPS (~$3.50–5/mo) so the OpenDex community chat moves from `#opendex:matrix.org` to `#opendex:opendex.org` on your own infrastructure.

## Prerequisites

- **VPS**: Ubuntu 24.04 LTS, 1 CPU, 1–2 GB RAM, 10–25 GB SSD
- **Domain**: A domain or subdomain (e.g., `matrix.opendex.org`) pointing to your VPS IP
- **SSH access** with sudo privileges
- **Existing web portal** (optional): If co-locating with the OpenDex static portal, ensure 2 GB+ RAM

## Quick Start

### One-Command Setup

```bash
# SSH into your VPS
ssh root@<your-vps-ip>

# Clone the repository (if not already present)
git clone <repo-url> /home/opendex/opendex

# Run the automated setup script
sudo bash docs/matrix/setup-matrix.sh matrix.opendex.org
```

The script will:
1. Install Conduit from the official release
2. Create a systemd service for auto-start
3. Generate the Conduit configuration file
4. Configure Nginx as a reverse proxy for federation traffic
5. Set up HTTPS via Let's Encrypt (Certbot)
6. Configure UFW firewall rules

### Manual Step-by-Step

#### 1. Install Conduit

```bash
# Download the latest Conduit release
CONDUIT_VERSION=$(curl -s https://api.github.com/repos/conduit-rms/conduit/releases/latest | grep tag_name | cut -d'"' -f4)
wget "https://github.com/conduit-rms/conduit/releases/download/${CONDUIT_VERSION}/conduit-x86_64-unknown-linux-musl.tar.gz"

# Extract and install
tar xzf conduit-x86_64-unknown-linux-musl.tar.gz
sudo mv conduit /usr/local/bin/
sudo chmod +x /usr/local/bin/conduit

# Verify installation
conduit --version
```

#### 2. Create Conduit User and Directories

```bash
# Create a dedicated user for Conduit
sudo useradd -r -s /bin/false -m -d /var/lib/conduit conduit

# Create required directories
sudo mkdir -p /var/lib/conduit
sudo mkdir -p /etc/conduit
sudo chown -R conduit:conduit /var/lib/conduit
sudo chown -R conduit:conduit /etc/conduit
```

#### 3. Configure Conduit

Copy the configuration template and edit it:

```bash
sudo cp docs/matrix/conduit.toml /etc/conduit/conduit.toml
sudo nano /etc/conduit/conduit.toml
```

Key settings to customize:
- `server_name`: Set to your domain (e.g., `matrix.opendex.org`)
- `registration_shared_secret`: Generate a strong random secret
- `database_url`: RocksDB path (default is `/var/lib/conduit/db`)

#### 4. Create Systemd Service

```bash
sudo nano /etc/systemd/system/conduit.service
```

Paste the following:

```ini
[Unit]
Description=Conduit Matrix Homeserver
After=network.target

[Service]
Type=simple
User=conduit
Group=conduit
ExecStart=/usr/local/bin/conduit --config /etc/conduit/conduit.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable conduit
sudo systemctl start conduit
sudo systemctl status conduit
```

#### 5. Configure Nginx Reverse Proxy

Install Nginx if not already present:

```bash
sudo apt install -y nginx
```

Copy the Nginx configuration:

```bash
sudo cp docs/matrix/nginx-matrix.conf /etc/nginx/sites-available/matrix
sudo ln -sf /etc/nginx/sites-available/matrix /etc/nginx/sites-enabled/matrix
sudo rm -f /etc/nginx/sites-enabled/default
```

Edit the configuration to match your domain:

```bash
sudo nano /etc/nginx/sites-available/matrix
```

Test and reload:

```bash
sudo nginx -t
sudo systemctl reload nginx
```

#### 6. Set Up HTTPS with Let's Encrypt

```bash
sudo apt install -y certbot python3-certbot-nginx

# Obtain and install the certificate
sudo certbot --nginx -d matrix.opendex.org

# Follow the prompts:
# - Enter email address
# - Agree to terms
# - Choose to redirect HTTP to HTTPS (recommended)

# Test auto-renewal
sudo certbot renew --dry-run
```

#### 7. Configure Firewall

```bash
# Allow SSH, HTTP, HTTPS, and Matrix federation
sudo ufw allow OpenSSH
sudo ufw allow 'Nginx Full'
sudo ufw allow 8448/tcp  # Matrix federation
sudo ufw enable
```

#### 8. Create Your First Admin User

```bash
# Register the first user via the Conduit CLI
conduit register --config /etc/conduit/conduit.toml --username admin --password <strong-password> --admin
```

#### 9. Create the OpenDex Community Room

Using a Matrix client (e.g., Element):

1. Log in with your admin account at `https://matrix.opendex.org`
2. Create a new room named `#opendex:matrix.opendex.org`
3. Set room visibility to public
4. Configure room settings (name, topic, etc.)

## Migration from Matrix.org

To migrate from `#opendex:matrix.org` to your self-hosted room:

1. **Announce the migration**: Post in the old room about the move
2. **Create the new room**: On your Conduit server
3. **Update all links**: The web frontend and documentation should point to the new room
4. **Invite existing members**: Use the new room link `https://matrix.to/#/#opendex:matrix.opendex.org`
5. **Federate with Matrix.org**: Conduit automatically federates, so users on matrix.org can join your room

## Backup

A backup script is provided at `docs/matrix/backup.sh`. Set up daily backups:

```bash
# Make the script executable
chmod +x docs/matrix/backup.sh

# Set up a daily cron job
sudo crontab -e
# Add this line for daily backups at 2 AM
0 2 * * * /path/to/opendex/docs/matrix/backup.sh
```

The script backs up the RocksDB database to a local directory and optionally uploads to remote storage (S3-compatible).

## Monitoring

Check Conduit status:

```bash
sudo systemctl status conduit
sudo journalctl -u conduit -f
```

Check disk usage:

```bash
du -sh /var/lib/conduit/
```

## Troubleshooting

### Federation not working

- Verify port 8448 is open: `sudo ufw status`
- Check Nginx config: `sudo nginx -t`
- Ensure your domain's DNS `_matrix` record points to your server
- Test federation: `curl -v https://matrix.opendex.org:8448/.well-known/matrix/server`

### HTTPS issues

- Check Certbot logs: `sudo certbot certificates`
- Verify Nginx SSL config: `sudo nano /etc/nginx/sites-available/matrix`
- Test certificate: `curl -v https://matrix.opendex.org`

### Conduit won't start

- Check logs: `sudo journalctl -u conduit -n 50`
- Verify config: `conduit --config /etc/conduit/conduit.toml --check-config`
- Check permissions: `sudo chown -R conduit:conduit /var/lib/conduit /etc/conduit`

## Updating Conduit

```bash
# Stop the service
sudo systemctl stop conduit

# Download the latest version
CONDUIT_VERSION=$(curl -s https://api.github.com/repos/conduit-rms/conduit/releases/latest | grep tag_name | cut -d'"' -f4)
wget "https://github.com/conduit-rms/conduit/releases/download/${CONDUIT_VERSION}/conduit-x86_64-unknown-linux-musl.tar.gz"
tar xzf conduit-x86_64-unknown-linux-musl.tar.gz
sudo mv conduit /usr/local/bin/

# Restart
sudo systemctl start conduit
sudo systemctl status conduit
```

## Configuration Reference

See `conduit.toml` in this directory for a complete configuration template with comments explaining each setting.

## Nginx Configuration Reference

See `nginx-matrix.conf` in this directory for the reverse proxy configuration that handles:
- Matrix federation traffic on port 8448
- Client-server API on port 443
- WebSocket connections for real-time messaging
