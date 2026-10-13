# OpenDex Web Portal

Informational website for the OpenDex P2P decentralized exchange, built with Leptos (Rust → WebAssembly).

## What's Included

- **Hero section** — Project name, tagline, and quick action buttons
- **About** — What OpenDex is and how it works
- **Features** — Key capabilities (P2P network, atomic swaps, privacy, etc.)
- **Downloads** — Native desktop app download links for Linux, macOS, and Windows
- **Source Code** — Built-in source code browser with all project files, audit docs, and architecture specs — no external repository needed
- **Legal** — Legal disclaimer, risk disclosure, jurisdiction notice, privacy policy, terms of use, and contact information

## Prerequisites

- Rust 1.88+ (stable or nightly)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Trunk for building and serving: `cargo install trunk`

## Development

```bash
# Serve with live reload (watches for changes)
trunk serve --open

# Or build for production
trunk build --release
```

The built files will be in `dist/` and can be deployed to a standard web server or static hosting service.

## Updating Download Links

Edit the `DownloadSection` function in `src/lib.rs` to update the download URLs. Place the actual binary files in the `static/releases/` directory.

## Updating Content

All content is in `src/lib.rs`. Each section is a standalone function — edit the text directly.

Legal and compliance content is in `src/legal.rs`. Edit the appropriate section to update legal content.

## Styling

CSS is in `static/style.css`. The site uses a dark theme with CSS custom properties for easy customization.

## Deployment

All deployment files — the guide, build scripts, and Nginx config — are centralized in **[docs/deployment/](../../docs/deployment/)**. No deployment scripts are kept in this crate directory.

### Quick Deploy to a VPS

```bash
# From the project root
cd docs/deployment

# Build for production
./build.sh --release

# Deploy to your server (one command)
./deploy.sh user@your-server.com
```

### Full Deployment Guide

See **[docs/deployment/DEPLOY.md](../../docs/deployment/DEPLOY.md)** for complete instructions covering:

- VPS setup (Ubuntu 22.04/24.04)
- Nginx configuration with SPA routing
- HTTPS with Let's Encrypt (Certbot)
- Alternative: Caddy with automatic HTTPS
- Alternative: Any static hosting service
- Troubleshooting common issues
- Security checklist
- Regulatory compliance guidelines

### What You Need

1. **A VPS** — Linode, Vultr, AWS Lightsail, etc. (~$4-6/mo)
2. **A domain name** (recommended — required for HTTPS)
3. **Rust + Trunk** installed locally to build the site

The site is fully static — no backend, no database, no runtime dependencies on the server.
