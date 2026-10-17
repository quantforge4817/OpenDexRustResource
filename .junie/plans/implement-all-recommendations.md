---
sessionId: session-261017-093825-7ldz
---

# Requirements

### Overview
Implement all 37 feature recommendations to make OpenDex better, organized into six categories: Quick Wins, Trading Enhancements, User Experience, Network & P2P, Security & Trust, and Community & Growth.

### Scope

#### In Scope - Website (web-frontend)
1. **FAQ Section** — Common questions about atomic swaps, wallets, reputation
2. **Blog/News Section** — Development updates and educational content
3. **Security Audit Badge** — Prominent display of audit status, test counts, Kani proofs
4. **Bug Bounty Program Page** — Vulnerability reporting, rewards, scope
5. **Video Tutorials Section** — Embedded tutorial placeholders for installation, first trade, wallets, reputation

#### In Scope - Desktop GUI (desktop-gui)
6. **Bootstrap Node List** — Built-in well-known nodes for one-click network join
7. **Counterparty Reputation Preview** — Trust score and tier badge inline in order book
8. **Order History** — Past orders with timestamps, prices, outcomes
9. **Trade Confirmation Dialog** — Summary before placing orders
10. **Price Alerts** — Notify when BTC/XMR reaches target price
11. **Order Book Depth Visualization** — Cumulative bid/ask bars
12. **Price Chart** — Recent trade prices over time
13. **Trade Notifications** — Desktop notifications on order match or state change
14. **Search and Filter in Order Book** — Filter by price range, amount, reputation
15. **User Profile Page** — Username, email, wallets, reputation, settings
16. **Password Reset Flow** — Email-based password recovery
17. **Dark/Light Theme Toggle** — Manual override of auto-detected theme
18. **Keyboard Shortcuts** — Ctrl+N, Ctrl+P, Ctrl+W, etc.
19. **Trade Escrow Visualization** — Visual swap state with confirmations
20. **On-Chain Transaction Links** — Clickable blockchain explorer links
21. **Personal Trading Analytics** — Volume, win rate, profit/loss stats
22. **Export Trade History** — CSV export for tax reporting
23. **Network Statistics Dashboard** — Total peers, volume, trade counts

#### Out of Scope (require external services or major infrastructure)
- Real email SMTP integration (requires SMTP server credentials)
- Two-Factor Authentication (requires TOTP library and production auth backend)
- Mobile app / responsive web client (separate project)
- Flatpak/Snap packaging (CI/CD infrastructure)
- Auto-update mechanism (requires update server)
- Multi-market support (requires protocol changes)
- Tor/I2P integration (requires networking changes)
- Peer reputation system (requires P2P protocol changes)
- CI/CD pipeline (requires GitHub Actions setup)
- Database backend for users (requires migration from JSON)
- Monitoring & health checks (requires external monitoring)
- Referral system (requires network-wide protocol changes)
- Fees dashboard (requires active fee system)

### User Stories
- As a new user, I want to join the network with one click so I don't need to manually find peers
- As a trader, I want to see counterparty reputation before accepting an order so I can assess risk
- As a trader, I want to review my order history so I can track my activity
- As a cautious trader, I want a confirmation dialog before placing orders so I don't make mistakes
- As a user, I want to set price alerts so I'm notified when the market moves
- As a power user, I want keyboard shortcuts to navigate the app quickly
- As a user, I want to view my profile and update my wallet addresses
- As a user who forgot my password, I want to reset it via email
- As a trader, I want to see visual swap state during a trade so I understand what's happening
- As a user, I want to export my trade history as CSV for tax reporting
- As a visitor, I want to read FAQs so I can understand how OpenDex works
- As a developer, I want to see security audit status so I can trust the codebase
- As a security researcher, I want to know how to report vulnerabilities responsibly


# Delivery Steps

### ✓ Step 1: Add website content sections: FAQ, Blog, Security Badge, Bug Bounty, Tutorials
Add five new informational sections to the static website:
- FAQ section with common questions about atomic swaps, wallets, reputation, and installation
- Blog/News section with development update placeholders
- Security Audit Badge section showing test counts, Kani proofs, and audit status
- Bug Bounty Program page with responsible disclosure guidelines
- Video Tutorials section with embedded tutorial placeholders
- Files: crates/web-frontend/src/lib.rs, crates/web-frontend/static/style.css

### ✓ Step 2: Bootstrap node list and counterparty reputation preview in order book
Make onboarding easier and build trust before trades:
- Add built-in bootstrap node list in peer management panel for one-click network join
- Show counterparty trust score and tier badge inline in order book rows
- Add reputation column to order book view
- Files: crates/desktop-gui/src/view/peers.rs, crates/desktop-gui/src/view/order_book.rs, crates/desktop-gui/src/model.rs

### ✓ Step 3: Order history panel and trade confirmation dialog
Essential trading features for tracking and safety:
- Add order history panel showing past orders with timestamps, prices, and outcomes
- Add confirmation dialog before placing orders summarizing price, amount, counterparty reputation
- Add new message types for order history and confirmation flow
- Files: crates/desktop-gui/src/app.rs, crates/desktop-gui/src/model.rs, crates/desktop-gui/src/view/order_book.rs, crates/desktop-gui/src/view/trade_form.rs

### ✓ Step 4: Price alerts, order book filtering, and depth visualization
Market analysis and monitoring tools:
- Add price alert system with configurable target prices and notification
- Add search/filter controls in order book for price range, amount, and reputation tier
- Add visual depth chart showing cumulative bid/ask bars
- Files: crates/desktop-gui/src/app.rs, crates/desktop-gui/src/model.rs, crates/desktop-gui/src/view/order_book.rs

### ✓ Step 5: User profile page, password reset, and theme toggle
Account management and personalization:
- Add dedicated profile page showing username, email, wallet addresses, reputation stats
- Add password reset flow via email verification
- Add manual dark/light theme toggle in settings
- Files: crates/desktop-gui/src/app.rs, crates/desktop-gui/src/model.rs, crates/desktop-gui/src/view/auth.rs, crates/desktop-gui/src/style.rs

### ✓ Step 6: Keyboard shortcuts, trade visualization, and blockchain links
Power user features and trade transparency:
- Add keyboard shortcuts: Ctrl+N (new order), Ctrl+P (peers), Ctrl+W (wallet), Ctrl+H (history)
- Add visual swap state display showing locked assets, confirmations, and progress
- Add clickable blockchain explorer links for Bitcoin and Monero transactions
- Files: crates/desktop-gui/src/app.rs, crates/desktop-gui/src/view/trades.rs, crates/desktop-gui/src/view/trade_form.rs

### ✓ Step 7: Trading analytics, CSV export, and network statistics dashboard
Analytics and network health monitoring:
- Add personal trading analytics panel: total volume, win rate, profit/loss, active hours
- Add CSV export of trade history for tax reporting
- Add network statistics dashboard: total peers, 24h volume, average trade size
- Files: crates/desktop-gui/src/app.rs, crates/desktop-gui/src/model.rs, crates/desktop-gui/src/view/trades.rs, crates/desktop-gui/src/view/peers.rs