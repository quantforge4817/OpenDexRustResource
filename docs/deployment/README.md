# OpenDex Web Portal — Deployment

This folder contains everything needed to build and deploy the OpenDex web portal.

## Files

| File | Description |
|------|-------------|
| `DEPLOYMENT_PLAN.md` | Complete deployment plan with regulatory compliance requirements |
| `DEPLOY.md` | Step-by-step deployment guide |
| `build.sh` | One-command build script (runs Trunk from the web-frontend crate) |
| `deploy.sh` | Automated deployment script (builds, uploads, configures Nginx) |
| `deploy/nginx.conf` | Nginx configuration for SPA routing, gzip, and security headers |

## Quick Start

```bash
# Build the site
./build.sh --release

# Deploy to a VPS
./deploy.sh user@your-server.com
```

## Full Guide

See [DEPLOY.md](DEPLOY.md) for detailed instructions covering:

- VPS provisioning and setup
- Domain configuration
- Nginx installation and configuration
- HTTPS with Let's Encrypt
- Alternative hosting (Caddy, static hosting providers)
- Troubleshooting

## Project Structure

```
docs/deployment/
├── DEPLOYMENT_PLAN.md # Deployment plan with compliance requirements
├── DEPLOY.md          # Deployment guide
├── README.md          # This file
├── build.sh           # Build script
├── deploy.sh          # Deploy script
└── deploy/
    └── nginx.conf     # Nginx config
```

The built static files will appear in `docs/deployment/dist/` after running `build.sh`.
