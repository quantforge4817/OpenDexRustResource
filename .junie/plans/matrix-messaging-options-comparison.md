---
sessionId: session-261017-183939-tkdi
---

# Option A: Cloud VPS Matrix Server — Design Plan

### Overview

Deploy a self-hosted Conduit Matrix homeserver on a low-budget cloud VPS (~$3.50–5/mo) for the OpenDex community chat, replacing the current `#opendex:matrix.org` room with `#opendex:opendex.org` on your own infrastructure.

### Goals

- Provide a reliable, low-cost Matrix messaging solution for OpenDex community communication
- Full data sovereignty — all messages, media, and metadata on your server
- Privacy alignment — matches OpenDex's mission of sovereignty and decentralization
- Minimal operational overhead — Conduit requires little maintenance
- Brand identity — `@user:opendex.org` handles instead of `@user:matrix.org`

### Scope

#### In Scope
- Conduit homeserver deployment on a cloud VPS
- Domain setup with HTTPS (Certbot/Let's Encrypt)
- Nginx reverse proxy for federation traffic
- Automated setup scripts
- Backup scripts for RocksDB
- Web frontend link update
- Deployment documentation

#### Out of Scope
- Element Web widget embedded in the static site (can be added later)
- Matrix bridges (Slack, Discord, etc.)
- Custom Matrix bots
- External media backend (e.g., S3) — local storage is sufficient for community chat

### VPS Specifications

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 1 core | 2 cores |
| RAM | 1 GB (Conduit only) | 2 GB (Conduit + static portal co-located) |
| Disk | 10 GB SSD | 25 GB SSD |
| OS | Ubuntu 24.04 LTS | Ubuntu 24.04 LTS |
| Provider | Vultr $3.50/mo, Linode $5/mo, AWS Lightsail $3.50/mo | Any |

### Architecture

```
┌─────────────────────────────────────────┐
│            VPS (Ubuntu 24.04)           │
│                                         │
│  ┌──────────────┐  ┌─────────────────┐ │
│  │  Web Portal   │  │  Conduit        │ │
│  │  Nginx        │  │  :6167 (local)  │ │
│  │  :80 / :443  │  │  Federation     │ │
│  └──────────────┘  │  :8448 (public) │ │
│                     └─────────────────┘ │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │  RocksDB Database                │  │
│  │  /var/lib/conduit/               │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Key Decisions

- **Conduit over Synapse**: Conduit is a single Rust binary with RocksDB backend — ~1GB RAM vs Synapse's 4-8GB with PostgreSQL. Conduit is designed for small VPS deployment.
- **RocksDB backend**: No separate database needed — embedded, fast, low maintenance.
- **systemd service**: Simpler than Docker for a single binary; easier updates and monitoring.
- **Same VPS or separate**: If the static portal VPS has 2GB+ RAM, co-locate Conduit with the existing web portal. Otherwise, use a separate $5/mo VPS.
- **Auto-updates enabled**: Conduit can check for and apply security updates automatically.
- **Domain strategy**: Use `matrix.opendex.org` subdomain for the Matrix server, or use the main domain `opendex.org` as the server name with a delegated `_matrix` host.

### Current Implementation

- **Web frontend**: `crates/web-frontend/` — Leptos/WASM static site, built with Trunk
- **Contact section**: `src/lib.rs:718-802` — links to `https://matrix.to/#/#opendex:matrix.org`
- **Deployment**: `docs/deployment/` — build script, deploy script, Nginx config
- **Testnet infra**: `docs/testnet/` — VPS setup scripts, systemd services

### New Files

| File | Purpose |
|------|---------|
| `docs/matrix/README.md` | Matrix server deployment guide |
| `docs/matrix/setup-matrix.sh` | Automated VPS setup script |
| `docs/matrix/conduit.toml` | Conduit configuration template |
| `docs/matrix/nginx-matrix.conf` | Nginx config for Matrix federation |
| `docs/matrix/backup.sh` | RocksDB backup script |

### Updated Files

| File | Change |
|------|--------|
| `crates/web-frontend/src/lib.rs` | Update Matrix link from `#opendex:matrix.org` to `#opendex:opendex.org` |
| `docs/deployment/DEPLOY.md` | Add Matrix server deployment section |
| `crates/web-frontend/README.md` | Mention the Matrix infrastructure option |

### Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Single point of failure | Daily RocksDB backups to remote storage |
| Federation abuse | Rate limiting in Conduit config, UFW firewall |
| Message loss | Backups stored off-server |
| DDoS | Cloudflare proxy (free tier) in front of Matrix subdomain |
| Operational burden | Conduit requires minimal maintenance; documented runbook in `docs/matrix/` |

### Monthly Cost

| Scenario | Cost |
|----------|------|
| Static portal only (existing) | ~$5/mo |
| + Conduit on same VPS (2GB plan) | ~$10/mo total |
| + Conduit on separate VPS | ~$10/mo total |

# Delivery Steps

### ✓ Step 1: Create Matrix deployment documentation
Create `docs/matrix/` with all deployment assets for a Conduit homeserver on a cloud VPS:
- `README.md` — comprehensive guide covering Conduit installation, domain setup, HTTPS, room creation, and migration from Matrix.org
- `setup-matrix.sh` — automated VPS setup script (Conduit install, systemd service, domain config, Certbot HTTPS, Conduit config generation)
- `conduit.toml` — configuration template with server name, RocksDB backend, federation settings, and trusted servers
- `nginx-matrix.conf` — Nginx reverse proxy config for Matrix federation traffic (port 8448) and client-server API
- `backup.sh` — daily RocksDB backup script with remote storage support

### ✓ Step 2: Update web frontend and deployment docs
Update existing project files to reference the new self-hosted Matrix server:
- Update `crates/web-frontend/src/lib.rs` ContactSection to link to the new `#opendex:opendex.org` room
- Update `docs/deployment/DEPLOY.md` to include a Matrix server deployment section
- Update `crates/web-frontend/README.md` to mention the Matrix infrastructure option