-       --
sessionId: session-261017-183939-tkdi
---

# Requirements & Goals

### Overview

The OpenDex static web portal (`crates/web-frontend`) already references a Matrix room (`#opendex:matrix.org`) as the primary community chat channel. The goal is to design a **low-cost VPS-based Matrix messaging solution** that gives the OpenDex project control over its communication infrastructure.

### Current State

- The web frontend is a **static Leptos/WASM site** deployed via Nginx on Ubuntu VPS (~$4-6/mo)
- Contact section (`src/lib.rs` line 750-758) links to `https://matrix.to/#/#opendex:matrix.org`
- No self-hosted Matrix infrastructure exists yet
- Deployment scripts live in `docs/deployment/` and `docs/testnet/`

### Goals

- Provide a reliable, low-cost Matrix messaging solution for OpenDex community communication
- Minimize operational overhead (this is a crypto project, not an ops team)
- Maintain data sovereignty and privacy alignment with the project's values
- Integrate with the existing static portal deployment model


# Options Comparison

### Current Implementation

- **Web frontend**: `crates/web-frontend/` — Leptos/WASM static site, built with Trunk
- **Deployment**: `docs/deployment/` — build script, deploy script, Nginx config
- **Testnet infra**: `docs/testnet/` — VPS setup scripts, systemd services
- **Contact section**: `src/lib.rs:718-802` — links to `#opendex:matrix.org` on Matrix.org

### Proposed Changes (Option 1 — Self-Hosted Conduit)

#### New Infrastructure

1. **Conduit Homeserver** deployed via Docker on the existing VPS (or a separate $5/mo VPS)
2. **Domain setup**: new subdomain (e.g., `matrix.opendex.org`) with HTTPS via Certbot
3. **Room migration**: create `#opendex:opendex.org` on the new server, update all links
4. **Nginx reverse proxy** for Matrix federation traffic (port 8448)

#### New Files

| File | Purpose |
|------|---------|
| `docs/matrix/README.md` | Matrix server deployment guide |
| `docs/matrix/docker-compose.yml` | Conduit + Element Web container definitions |
| `docs/matrix/setup-matrix.sh` | Automated VPS setup script |
| `docs/matrix/nginx.conf` | Nginx config for Matrix federation + Element Web |
| `docs/matrix/backup.sh` | Database backup script |

#### Updated Files

| File | Change |
|------|--------|
| `crates/web-frontend/src/lib.rs` | Update Matrix link from `#opendex:matrix.org` to `#opendex:opendex.org` |
| `docs/deployment/DEPLOY.md` | Add Matrix server section |

### Key Decisions

- **Conduit over Synapse**: Conduit is written in Rust, uses ~10% of the RAM of Synapse, and is designed for low-resource VPS deployment. Synapse would require 4GB+ RAM.
- **SQLite backend**: No separate PostgreSQL needed — Conduit's SQLite mode is sufficient for community-scale usage.
- **Docker deployment**: Simplifies updates, backups, and isolation from the existing Nginx static portal setup.
- **Same VPS or separate**: If the static portal VPS has 2GB+ RAM, co-locate. Otherwise, use a separate $5/mo VPS.

### VPS Specifications

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 1 core | 2 cores |
| RAM | 1 GB (Conduit only) | 2 GB (Conduit + Element Web) |
| Disk | 10 GB SSD | 25 GB SSD |
| OS | Ubuntu 24.04 LTS | Ubuntu 24.04 LTS |
| Provider | Vultr $3.50/mo, Linode $5/mo, AWS Lightsail $3.50/mo | Any |

### Monthly Cost Breakdown

| Scenario | Cost |
|----------|------|
| Static portal only (existing) | ~$5/mo |
| + Conduit on same VPS (2GB plan) | ~$10/mo total |
| + Conduit on separate VPS | ~$10/mo total |
| Option 2 (hosted, no server) | ~$5/mo total |

### Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Single point of failure | Regular backups via `backup.sh` to remote storage |
| Federation abuse | Rate limiting in Conduit config, UFW firewall rules |
| Message loss | Daily SQLite backups, stored off-server |
| DDoS | Cloudflare proxy (free tier) in front of Matrix subdomain |
| Operational burden | Conduit requires minimal maintenance; document runbook in `docs/matrix/` |

# Delivery Steps

###   Step 1: Create Matrix deployment documentation
Create the `docs/matrix/` directory with all deployment assets:
- `README.md` — comprehensive guide covering Conduit installation, domain setup, HTTPS, room creation, and migration from Matrix.org
- `docker-compose.yml` — Conduit homeserver container with SQLite backend, volume mounts, and port mappings (8448 for federation)
- `setup-matrix.sh` — automated VPS setup script (Docker install, domain config, Certbot HTTPS, Conduit config generation)
- `nginx.conf` — Nginx reverse proxy config for Matrix federation traffic and optional Element Web serving
- `backup.sh` — daily SQLite backup script with remote storage support

###   Step 2: Update web frontend and deployment docs
Update existing project files to reference the new self-hosted Matrix server:
- Update `crates/web-frontend/src/lib.rs` ContactSection to link to the new `#opendex:opendex.org` room
- Update `docs/deployment/DEPLOY.md` to include a Matrix server deployment section
- Update `crates/web-frontend/README.md` to mention the Matrix infrastructure option