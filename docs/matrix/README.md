# OpenDex Matrix Server Deployment Guide

This guide walks you through deploying a **Conduit** Matrix server on a low-cost cloud VPS (~$3.50–5/mo) for the OpenDex community chat. This replaces reliance on `matrix.org` with an OpenDex-owned server (`#opendex:opendex.org`), providing full data sovereignty and custom `@user:opendex.org` handles.

## Why Conduit?

| Feature | Conduit | Synapse |
|---------|---------|---------|
| Language | Rust | Python |
| RAM Usage | ~100–500 MB | 2–4 GB+ |
| Min VPS Cost | ~$3.50/mo | ~$20/mo |
| Performance | Excellent | Good |
| Maintenance | Minimal | Moderate |

Conduit is written in Rust, uses a fraction of the resources of Synapse, and is ideal for low-cost VPS deployment.

## Prerequisites

1. **A cloud VPS** — Vultr ($3.50/mo), Host4Fun ($3/mo annual), or VPSMart ($4.50/mo)
   - Ubuntu 24.04 LTS
   - 1–2 GB RAM, 1 core, 10–25 GB SSD
2. **A domain name** — e.g., `opendex.org` with DNS access
3. **SSH access** to the VPS
4. **Docker** — installed by the setup script

## Quick Start

```bash
# SSH into your VPS
ssh user@your-vps-ip

# Clone the OpenDex repository (or copy just the docs/matrix folder)
git clone https://github.com/OpenDex/OpenDexRustSource.git
cd OpenDexRustSource/docs/matrix

# Run the automated setup script (replace with your domain)
sudo bash setup-matrix.sh matrix.opendex.org
```

The script will:
1. Install Docker and Docker Compose
2. Configure the domain DNS (`.well-known` discovery)
3. Set up Nginx as a reverse proxy
4. Obtain HTTPS certificates via Certbot
5. Generate the Conduit configuration
6. Start the Conduit server

## Manual Setup

If you prefer manual control, follow these steps:

### 1. Install Docker

```bash
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker $USER
```

### 2. Configure DNS

Add the following DNS records for your domain:

| Type | Name | Value |
|------|------|-------|
| A | matrix.opendex.org | <your-vps-ip> |
| TXT | _matrix._tcp.matrix.opendex.org | matrix://matrix.opendex.org |

### 3. Set Up Nginx

```bash
sudo apt install nginx certbot python3-certbot-nginx
sudo cp nginx-matrix.conf /etc/nginx/sites-available/matrix
sudo ln -s /etc/nginx/sites-available/matrix /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl restart nginx
```

### 4. Obtain HTTPS Certificate

```bash
sudo certbot --nginx -d matrix.opendex.org
```

### 5. Configure Conduit

Edit `conduit.toml` with your settings:

```toml
server_name = "opendex.org"
# ... see conduit.toml for full configuration
```

### 6. Start Conduit

```bash
docker compose up -d
```

### 7. Verify Installation

```bash
# Check Conduit is running
docker compose ps

# Check the server is accessible
curl -s https://matrix.opendex.org/.well-known/matrix/server | jq

# Create an admin user
curl -X POST http://localhost:6167/_matrix/client/r0/admin/register \
  -u admin:your-password \
  -d '{"username": "admin", "password": "your-password", "admin": true, "displayname": "Admin"}'
```

## Room Creation

After creating an admin user, create the community room:

```bash
# Login to get an access token
curl -X POST https://matrix.opendex.org/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{"type": "m.login.password", "user": "admin", "password": "your-password"}'

# Create the room
curl -X POST https://matrix.opendex.org/_matrix/client/v3/createRoom \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"room_alias_name": "opendex", "name": "OpenDex Community", "topic": "Official OpenDex community chat", "visibility": "public"}'
```

## Migration from matrix.org

To migrate from `#opendex:matrix.org` to `#opendex:opendex.org`:

1. **Announce the migration** in the old room well in advance
2. **Create the new room** on your Conduit server
3. **Update all links** in project documentation, website, and social media
4. **Set up federation** — Conduit can federate with matrix.org, so users on both servers can communicate
5. **Invite key members** to the new room
6. **Archive the old room** after a transition period

## File Structure

| File | Purpose |
|------|---------|
| `README.md` | This deployment guide |
| `docker-compose.yml` | Conduit container configuration |
| `setup-matrix.sh` | Automated VPS setup script |
| `conduit.toml` | Conduit configuration template |
| `nginx-matrix.conf` | Nginx reverse proxy configuration |
| `backup.sh` | Daily RocksDB backup script |

## Daily Backups

The included `backup.sh` script backs up the Conduit RocksDB database:

```bash
# Run manually
sudo bash backup.sh

# Or set up a cron job
sudo crontab -e
# Add: 0 2 * * * /path/to/backup.sh
```

Backups are stored in `/var/backups/conduit/` and can be synced to remote storage.

## Security Checklist

- [ ] Firewall configured (UFW) — allow only ports 22, 80, 443, 8448
- [ ] HTTPS enabled with valid certificate
- [ ] SSH key authentication only (disable password auth)
- [ ] Regular system updates
- [ ] Fail2ban for SSH protection
- [ ] Conduit registration tokens secured
- [ ] Daily backups configured and tested
- [ ] Cloudflare proxy enabled (optional but recommended)

## Troubleshooting

### Conduit won't start

```bash
# Check logs
docker compose logs conduit

# Verify config
docker compose config
```

### Federation issues

```bash
# Check .well-known endpoint
curl -s https://matrix.opendex.org/.well-known/matrix/server

# Verify DNS
dig _matrix._tcp.matrix.opendex.org TXT
```

### Certificate issues

```bash
# Renew certificate
sudo certbot renew

# Test renewal
sudo certbot renew --dry-run
```

## Cost Breakdown

| Item | Monthly Cost |
|------|-------------|
| VPS (1–2 GB RAM) | ~$3.50–5/mo |
| Domain name | ~$10/year |
| **Total** | **~$4.50–6/mo** |

## Support

For issues or questions:

- **Conduit documentation**: https://conduit-rust.github.io/conduit/
- **Conduit Matrix room**: #conduit:conduit-rust.net
- **OpenDex community**: #opendex:opendex.org

## License

This deployment guide is part of the OpenDex project, licensed under MIT or Apache-2.0.
