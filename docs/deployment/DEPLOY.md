# OpenDex Web Portal — Deployment Guide

This guide walks you through deploying the OpenDex web portal to a VPS (Virtual Private Server) with HTTPS.

---

## Quick Summary

The web portal is a **static site** built with Leptos + Trunk. You build it locally, then upload the `dist/` folder to any web server. No backend, no database, no runtime dependencies on the server.

**What you need:**
1. A VPS (Ubuntu 22.04/24.04 recommended)
2. A domain name (recommended — required for HTTPS)
3. Rust + Trunk installed locally to build the site

---

## Step 1: Get a VPS

Any cloud provider works. Popular options:

| Provider | Starting Price | Notes |
|----------|---------------|-------|
| **Linode/Akamai** | ~$5/mo | Reliable |
| **Vultr** | ~$3.50/mo | Many locations |
| **AWS Lightsail** | ~$3.50/mo | AWS ecosystem |

**Recommended specs:** 1 CPU, 1 GB RAM, 25 GB SSD is more than enough for a static site.

**OS:** Ubuntu 22.04 or 24.04 LTS (64-bit)

---

## Step 2: Point Your Domain (Optional)

If you have a domain name:

1. In your domain registrar's DNS settings, add an **A record**:
   - Type: `A`
   - Name: `@` (or your domain)
   - Value: `<your-VPS-IP>`
   - TTL: `300`

2. For `www` subdomain, add another A record:
   - Type: `A`
   - Name: `www`
   - Value: `<your-VPS-IP>`

If you don't have a domain, you can still deploy using the VPS IP address directly (but HTTPS via Let's Encrypt requires a domain).

---

## Step 3: Build the Site Locally

On your development machine:

```bash
# Install prerequisites (if not already done)
rustup target add wasm32-unknown-unknown
cargo install trunk

# Navigate to the deployment folder
cd docs/deployment

# Build for production
./build.sh --release
```

This creates a `dist/` directory containing all the static files (HTML, CSS, JS, WASM).

---

## Step 4: Set Up the VPS

SSH into your VPS:

```bash
ssh root@<your-vps-ip>
```

### 4.1: Initial Server Setup

```bash
# Update system
apt update && apt upgrade -y

# Create a non-root user (recommended)
adduser deploy
usermod -aG sudo deploy

# Switch to the new user
su - deploy

# Set up SSH keys (from your local machine)
mkdir -p ~/.ssh
chmod 700 ~/.ssh
nano ~/.ssh/authorized_keys
# Paste your public SSH key here
chmod 600 ~/.ssh/authorized_keys

# Set up firewall
ufw allow OpenSSH
ufw allow 'Nginx Full'
ufw enable
```

### 4.2: Install Nginx

```bash
sudo apt install -y nginx
sudo systemctl enable nginx
sudo systemctl start nginx
```

Verify Nginx is running:
```bash
sudo systemctl status nginx
```

Visit `http://<your-vps-ip>` in your browser — you should see the Nginx welcome page.

---

## Step 5: Deploy the Site

### Option A: Automated Deploy Script (Recommended)

From your local machine, in the `docs/deployment/` directory:

```bash
./deploy.sh deploy@<your-vps-ip-or-domain>
```

This script will:
1. Build the site in release mode
2. Upload the `dist/` contents to `/var/www/opendex/`
3. Install the Nginx configuration
4. Reload Nginx

### Option B: Manual Deployment

**On your local machine:**

```bash
# Build the site
cd docs/deployment
./build.sh --release

# Upload to VPS
rsync -avz --delete dist/ deploy@<your-vps-ip>:/var/www/opendex/
```

**On the VPS:**

```bash
# Create the web directory
sudo mkdir -p /var/www/opendex

# Set permissions
sudo chown -R www-data:www-data /var/www/opendex
sudo chmod -R 755 /var/www/opendex

# Install Nginx config
sudo nano /etc/nginx/sites-available/opendex
# Paste the contents of docs/deployment/deploy/nginx.conf (edit server_name)
sudo ln -sf /etc/nginx/sites-available/opendex /etc/nginx/sites-enabled/opendex

# Remove default site
sudo rm /etc/nginx/sites-enabled/default

# Test and reload
sudo nginx -t
sudo systemctl reload nginx
```

---

## Step 6: Set Up HTTPS (Let's Encrypt)

**Requires a domain name pointing to your VPS.**

```bash
# On the VPS
sudo apt install -y certbot python3-certbot-nginx

# Obtain and install the certificate
sudo certbot --nginx -d your-domain.com -d www.your-domain.com

# Follow the prompts:
# - Enter email address
# - Agree to terms
# - Choose to redirect HTTP to HTTPS (recommended)

# Test auto-renewal
sudo certbot renew --dry-run
```

Certbot automatically configures Nginx for HTTPS and sets up auto-renewal.

---

## Step 7: Verify the Deployment

1. Visit `https://your-domain.com` in your browser
2. Check the home page loads correctly
3. Navigate to the Source Code Browser (`/source`)
4. Verify the file tree and code viewer work
5. Check the browser console for any errors

---

## Updating the Site

Whenever you make changes to the web portal:

```bash
# Local machine, from docs/deployment/
./deploy.sh deploy@<your-vps-ip-or-domain>
```

Or manually:

```bash
./build.sh --release
rsync -avz --delete dist/ deploy@<your-vps-ip>:/var/www/opendex/
```

---

## Troubleshooting

### Site shows Nginx default page
- Check that `/etc/nginx/sites-enabled/default` is removed
- Run `sudo systemctl reload nginx`

### 403 Forbidden
- Check file permissions: `sudo chown -R www-data:www-data /var/www/opendex`
- Check directory permissions: `sudo chmod -R 755 /var/www/opendex`

### Source browser shows empty
- Verify the build completed: check `dist/` contains `.wasm` files
- Check browser console for WASM loading errors

### HTTPS not working
- Verify domain DNS points to VPS IP: `dig your-domain.com`
- Check Certbot logs: `sudo certbot certificates`
- Check Nginx error log: `sudo tail -f /var/log/nginx/error.log`

### Firewall blocking traffic
```bash
sudo ufw status
sudo ufw allow 'Nginx Full'
```

---

## Alternative: Caddy (Simpler HTTPS)

If you prefer automatic HTTPS without Certbot:

```bash
# Install Caddy
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install -y caddy

# Create Caddyfile
sudo nano /etc/caddy/Caddyfile
```

```
your-domain.com {
    root * /var/www/opendex
    file_server
    encode gzip
    try_files {path} /index.html
}
```

```bash
sudo systemctl restart caddy
```

Caddy automatically obtains and renews HTTPS certificates.

---

## Alternative: Static Hosting Providers

If you don't want a VPS, you can host the built static files on a standard web server or static hosting service:

```bash
# Build the site
cd docs/deployment
./build.sh --release
```

Then upload the contents of `dist/` to your hosting provider or web server.

---

## Optional: Matrix Server Deployment

If you want to run a self-hosted Matrix homeserver for the OpenDex community chat (replacing `#opendex:matrix.org` with `#opendex:opendex.org`), see the dedicated guide:

**[docs/matrix/README.md](../matrix/README.md)** — Complete deployment guide for a Conduit Matrix homeserver on a cloud VPS.

### Quick Start

```bash
# SSH into your VPS (same VPS as the web portal, or a separate one)
ssh root@<your-vps-ip>

# Run the automated setup script
sudo bash docs/matrix/setup-matrix.sh matrix.opendex.org
```

### What's Included

| File | Purpose |
|------|---------|
| `docs/matrix/README.md` | Full deployment guide |
| `docs/matrix/setup-matrix.sh` | Automated setup script |
| `docs/matrix/conduit.toml` | Conduit configuration template |
| `docs/matrix/nginx-matrix.conf` | Nginx reverse proxy config |
| `docs/matrix/backup.sh` | Daily RocksDB backup script |

### Co-locating with the Web Portal

If your VPS has **2 GB+ RAM**, you can run both the static web portal and the Matrix server on the same machine. The Matrix server (Conduit) uses ~1 GB RAM, leaving enough for Nginx and the static site.

### VPS Requirements for Matrix

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 1 core | 2 cores |
| RAM | 1 GB (Matrix only) | 2 GB (Matrix + web portal) |
| Disk | 10 GB SSD | 25 GB SSD |
| Cost | ~$3.50–5/mo | ~$5–10/mo |

---

## Security Checklist

- [ ] Firewall configured (UFW)
- [ ] HTTPS enabled with valid certificate
- [ ] SSH key authentication only (disable password auth)
- [ ] Regular system updates: `sudo apt update && sudo apt upgrade`
- [ ] Fail2ban for SSH protection: `sudo apt install fail2ban`
- [ ] Nginx security headers configured (included in nginx.conf)
- [ ] Content-Security-Policy header active (WASM-enabled)
- [ ] Legal page accessible at `/legal`
- [ ] Privacy policy and terms of use are current

---

## Regulatory Compliance

The OpenDex web portal is designed to be **informational only**, consistent with the regulatory compliance architecture documented in the project's audit files.

### Website Design Principles

The website must **not** be modified to:
- Accept orders or trade requests
- Route trades or provide a trading interface
- Act as a Crypto-Asset Service Provider (CASP)
- Create any argument that OpenDex operates as a financial service

### Compliance Documentation

Full regulatory compliance documentation is available through the built-in source browser:

- **Regulatory compliance master document**: `/source#audit/regulatory_compliance/regulatory_compliance.md`
- **Architecture specifications**: `/source#audit/`
- **Security audit reports**: `/source#audit/`

### Legal Page

The website includes a dedicated legal page at `/legal` containing:
- Legal disclaimer
- Risk disclosure (cryptocurrency, Monero regulatory, technical, market risks)
- Jurisdiction notice (EU MiCA, UK FCA, other jurisdictions)
- Privacy policy (no data collection, P2P protocol privacy)
- Terms of use (informational service, no financial service, limitation of liability)
- Contact information (general inquiries, security vulnerabilities)

### Deployment Responsibility

The person deploying the website is responsible for:
- Ensuring the legal page content is current and accurate
- Verifying that the website does not provide any trading functionality
- Maintaining the regulatory compliance documentation in the source browser
- Consulting qualified legal counsel before deploying in regulated jurisdictions
- Updating contact information with valid email addresses

### MiCA and FCA Considerations

- **EU MiCA Article 76**: Privacy coins (including Monero) may be restricted. The website's risk disclosure addresses this.
- **UK FCA**: Substance-based perimeter analysis applies. The website is designed to avoid creating a CASP argument.
- **General**: The website provides information only; it does not constitute financial or legal advice.
