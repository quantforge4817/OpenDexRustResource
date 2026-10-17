# OpenDex Web Portal — Deployment Plan

This document outlines the complete deployment plan for the OpenDex web portal, including regulatory compliance requirements, infrastructure architecture, and deployment procedures.

---

## Table of Contents

1. [Overview](#overview)
2. [Production Architecture](#production-architecture)
3. [Desktop Application](#desktop-application)
4. [Regulatory Compliance Requirements](#regulatory-compliance-requirements)
5. [Infrastructure Requirements](#infrastructure-requirements)
6. [Deployment Procedures](#deployment-procedures)
7. [Security Configuration](#security-configuration)
8. [Compliance Verification Checklist](#compliance-verification-checklist)
9. [Maintenance and Updates](#maintenance-and-updates)

---

## Overview

The OpenDex Web Portal is a static informational website for the OpenDex P2P decentralized exchange. It provides project information, software downloads, built-in source code browsing, and legal compliance documentation.

### Website Purpose

The website is designed to be **informational only**. It does not:
- Accept orders or trade requests
- Route trades or provide a trading interface
- Act as a Crypto-Asset Service Provider (CASP)
- Create any argument that OpenDex operates as a financial service

This design is consistent with the regulatory compliance architecture documented in `audit/regulatory_compliance/regulatory_compliance.md`.

### What's Included

| Section | Description |
|---------|-------------|
| Hero | Project name, tagline, and primary actions |
| About | What OpenDex is and how it works |
| Features | Key capabilities including P2P networking, atomic swaps, and privacy |
| Downloads | Native OpenDex desktop application downloads for Linux, macOS, and Windows |
| Source Code | Built-in source code browser containing project files, audit documentation, and architecture specifications |
| Legal | Legal disclaimer, risk disclosure, jurisdiction notice, privacy policy, terms of use, and contact information |

---

## Production Architecture

```
OpenDex Source
      │
      ▼
Rust + Leptos
      │
      ▼
    Trunk
      │
      ▼
  dist/
      │
      ▼
 OpenDex VPS
      │
      ▼
 Nginx / Caddy
      │
      ▼
 OpenDex Web Portal
```

The deployed website is fully static. No OpenDex backend, database, or server-side application runtime is required for the web portal.

---

## Desktop Application

The OpenDex Desktop Application is a standalone desktop client that embeds both the P2P node and the GUI in a single process. It runs on Linux, Windows, and macOS.

### Architecture

```
opendex-desktop (binary)
│
├── tokio runtime (main thread)
│   ├── P2P Node (p2p-node crate)
│   │   ├── libp2p swarm
│   │   ├── Order book
│   │   ├── Trade book
│   │   ├── Balance book
│   │   └── Wallet clients (optional)
│   │
│   └── Bridge adapters
│       ├── GUI commands → Node bridge
│       └── Node updates → GUI
│
└── iced GUI thread
    ├── Status bar
    ├── Order book panel
    ├── Trade form
    ├── Trades panel
    ├── Peers panel
    └── Wallet panel
```

### Features

- **Order Book** — View current bids and asks
- **Trade Form** — Place buy/sell orders with price and amount
- **Trade History** — Monitor active trades and their state
- **Wallet Panel** — View balances and fund test accounts
- **Peer Management** — Connect to P2P peers
- **Node Status Bar** — Monitor node identity and connections
- **On-chain Wallets** — Optional Monero and Bitcoin wallet integration

### Regulatory Compliance

The desktop GUI is rated **🟢 GREEN** under the OpenDex regulatory compliance framework:

- The GUI is a local node client, not a gateway to a central exchange
- No orders are routed through a central service
- No user data is collected or transmitted to third parties
- All P2P communication is encrypted via libp2p

### Building

**Linux:**
```bash
./crates/desktop-gui/build/build-linux.sh --release
```

**Windows:**
```cmd
crates\desktop-gui\build\build-windows.bat release
```

### Running

```bash
cargo run -p desktop-gui
```

See `crates/desktop-gui/README.md` for full documentation.

---

## Regulatory Compliance Requirements

Based on the review of `audit/regulatory_compliance/regulatory_compliance.md` (EU MiCA and UK FCA regimes), the following compliance requirements have been implemented.

### 1. Legal Disclaimer (Implemented)

**Regulatory basis:** The compliance document itself opens with a disclaimer stating these documents do not constitute legal opinions. The website, as the primary public-facing interface, requires an equivalent disclaimer.

**Implementation:** A prominent legal disclaimer section is available at `/legal` stating:
- The website provides informational content only
- Nothing on the site constitutes financial, legal, or investment advice
- Users should consult qualified regulatory counsel in their jurisdiction
- The project makes no representation about the legality of using OpenDex in any specific jurisdiction

### 2. Risk Disclosure (Implemented)

**Regulatory basis:** Section 5.24 (AMBER/RED — XMR Trading) notes that XMR trading creates additional regulatory complexity under MiCA Article 76. Section 5.17 (AMBER/RED — OpenDex Website) warns that the website's regulatory status depends on what it does.

**Implementation:** A risk disclosure section at `/legal#risk-disclosure` covers:
- Cryptocurrency trading involves substantial risk of loss
- Monero (XMR) may be restricted or prohibited in certain jurisdictions under MiCA Article 76
- Users are responsible for understanding the regulatory status of crypto-asset trading in their jurisdiction
- Atomic swaps are experimental technology with smart contract and cryptographic risks

### 3. Jurisdiction Notice (Implemented)

**Regulatory basis:** Section 1.1.3 (UK FCA Analysis) and Section 2.25 (UK FCA Design Objective) emphasize that the UK and EU have different regulatory treatments. The FCA's perimeter analysis is "substance-based and fact-specific."

**Implementation:** A jurisdiction notice at `/legal#jurisdiction` states:
- OpenDex availability may vary by jurisdiction
- Users in regulated jurisdictions (EU, UK, etc.) should verify compliance with local laws
- The project does not restrict access by jurisdiction but users bear responsibility for compliance
- Specific guidance for EU MiCA and UK FCA regimes

### 4. Privacy Policy (Implemented)

**Regulatory basis:** The compliance document extensively discusses data handling, user privacy, and the absence of central logs. The website claims "No KYC, no central logs, no tracking" — this is a privacy claim that should be formalized.

**Implementation:** A privacy policy at `/legal#privacy` covers:
- What data the website collects (none — it is a static site)
- No personal data is collected or stored
- No tracking cookies or analytics
- No KYC or identity verification
- How the P2P protocol handles privacy (libp2p encryption, Monero privacy features)

### 5. Terms of Use (Implemented)

**Regulatory basis:** Section 2.25 states the FCA considers "who contracts with users" as a relevant factor. Section 5.17 warns against the website creating a CASP argument.

**Implementation:** Terms of use at `/legal#terms` cover:
- The website is informational only; it does not accept orders, route trades, or provide a trading interface
- OpenDex software is provided "as is" without warranty
- Users accept full responsibility for their use of the software
- The project is not a financial service provider
- Limitation of liability

### 6. Contact Information (Implemented)

**Regulatory basis:** Regulatory frameworks typically require identifiable contact information for public-facing services.

**Implementation:** A contact section at `/legal#contact` provides:
- Contact email for general inquiries
- Separate email for security vulnerability reports
- Link to the source browser for documentation
- Link to the community Matrix chat at `#opendex:matrix.org`

### 7. Compliance Reference in Deployment Guide (Implemented)

**Implementation:** The deployment guide (`DEPLOY.md`) includes a regulatory compliance section noting:
- The website is designed to be informational only
- The website must not be modified to accept orders, route trades, or provide a trading interface without legal review
- Links to the compliance documentation in the source browser

### 8. Security Headers in Nginx Config (Implemented)

**Implementation:** The `nginx.conf` includes:
- `Content-Security-Policy` header (WASM-enabled) to prevent XSS
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Permissions-Policy` restricting geolocation, microphone, and camera
- HTTP-to-HTTPS redirect configuration

### 9. Source Browser — Compliance Docs Visibility (Implemented)

**Implementation:** The source browser includes all audit documentation, including regulatory compliance documents at `/source#audit/regulatory_compliance/`.

### 10. License Consistency (Implemented)

**Implementation:** The footer consistently displays "MIT OR Apache-2.0" across all pages.

---

## Infrastructure Requirements

### VPS Requirements

The OpenDex Web Portal requires minimal server capacity:

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| vCPU | 1 | 1–2 |
| RAM | 1 GB | 1–2 GB |
| Storage | 25 GB | 40 GB |
| OS | Debian 12 or Ubuntu LTS | Ubuntu 24.04 LTS |
| Network | Public IPv4 | Public IPv4 + IPv6 |

The website does not require Docker, a database, or a backend application server.

### Domain

A dedicated domain name is **recommended** for the public OpenDex website. HTTPS should be enabled for production deployments.

### Web Server

Nginx or Caddy is required to serve the static files. Nginx configuration is provided in `deploy/nginx.conf`.

---

## Deployment Procedures

### Prerequisites

- Rust 1.88+ (stable or nightly)
- `wasm32-unknown-unknown` target
- Trunk build tool

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

### Build

From the project root:

```bash
cd docs/deployment
./build.sh --release
```

The production files are generated in `docs/deployment/dist/`.

### Deploy to VPS

```bash
./deploy.sh user@your-server.com
```

The deployment script builds the production site and uploads the resulting static files to the configured web server.

### HTTPS Setup

On the VPS:

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d your-domain.com
```

### Full Deployment Guide

See `DEPLOY.md` for detailed instructions covering:
- VPS provisioning and setup
- Domain configuration
- Nginx installation and configuration
- Single-page application routing
- HTTPS with Let's Encrypt and Certbot
- Desktop application downloads
- Website updates
- Troubleshooting

---

## Security Configuration

### Nginx Security Headers

The provided `nginx.conf` includes the following security headers:

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Frame-Options` | `DENY` | Prevents clickjacking |
| `X-Content-Type-Options` | `nosniff` | Prevents MIME-type sniffing |
| `X-XSS-Protection` | `1; mode=block` | Enables XSS filter |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Controls referrer information |
| `Content-Security-Policy` | WASM-enabled policy | Prevents XSS and injection attacks |
| `Permissions-Policy` | Restricted | Disables unnecessary browser features |

### Firewall Configuration

```bash
sudo ufw allow OpenSSH
sudo ufw allow 'Nginx Full'
sudo ufw enable
```

### SSH Hardening

- Disable root login
- Use key-based authentication
- Configure fail2ban: `sudo apt install fail2ban`

---

## Compliance Verification Checklist

Before deploying to production, verify the following:

### Legal Compliance
- [ ] Legal disclaimer is accessible at `/legal`
- [ ] Risk disclosure section is current
- [ ] Jurisdiction notice covers EU MiCA and UK FCA
- [ ] Privacy policy accurately describes data collection (none)
- [ ] Terms of use clearly state informational-only service
- [ ] Contact information is up to date
- [ ] Legal page content has been reviewed by qualified legal counsel

### Technical Compliance
- [ ] Content-Security-Policy header is active (WASM-enabled)
- [ ] X-Frame-Options set to DENY
- [ ] HTTPS is enabled with valid certificate
- [ ] No tracking scripts or analytics are loaded
- [ ] No personal data is collected by the website
- [ ] Source browser includes regulatory compliance documentation

### Deployment
- [ ] VPS meets minimum requirements
- [ ] Domain name is configured and points to VPS
- [ ] Nginx is configured with security headers
- [ ] Firewall is properly configured
- [ ] Regular system updates are scheduled
- [ ] Fail2ban is installed for SSH protection

---

## Maintenance and Updates

### Updating Website Content

Website content is contained in `crates/web-frontend/src/lib.rs`. Each major section is implemented as a standalone function. Edit the appropriate section to update website content.

Legal content is in `crates/web-frontend/src/legal.rs`.

### Updating Download Links

Edit the `DownloadSection` function in `src/lib.rs` to update desktop application download URLs.

Desktop application release files may be placed in `static/releases/` when distributed directly from the web server.

### Updating Styling

Website styling is located in `static/style.css`. The site uses a dark theme with CSS custom properties for centralized styling configuration.

### Rebuilding and Redeploying

```bash
cd docs/deployment
./build.sh --release
./deploy.sh user@your-server.com
```

### Regulatory Compliance Updates

When regulatory requirements change:
1. Review `audit/regulatory_compliance/regulatory_compliance.md` for updated guidance
2. Update legal content in `src/legal.rs` as needed
3. Have changes reviewed by qualified legal counsel
4. Rebuild and redeploy the website

---

## Project Independence

The OpenDex Web Portal is designed to operate independently of any particular hosting provider or source-code hosting platform.

The website does not require:
- GitHub or any source code hosting platform
- A specific VPS provider
- A managed application platform
- Docker
- A database or backend server

The OpenDex project controls its own website deployment and infrastructure.

---

## File Structure

```
docs/deployment/
├── DEPLOYMENT_PLAN.md     # This file
├── DEPLOY.md              # Step-by-step deployment guide
├── README.md              # Quick overview
├── build.sh               # Build script
├── deploy.sh              # Deploy script
└── deploy/
    └── nginx.conf         # Nginx configuration
```

---

## References

- Regulatory compliance documentation: `audit/regulatory_compliance/regulatory_compliance.md`
- Web frontend source: `crates/web-frontend/`
- Web frontend README: `crates/web-frontend/README.md`
