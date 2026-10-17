# OpenDEX Master Change and Development Roadmap

**Document status:** Draft for review
**Version:** 0.2
**Document owner:** To be assigned
**Technical lead:** To be assigned
**Target release:** To be agreed
**Review frequency:** Weekly during active development

---

## 1. Executive Summary

This document defines the roadmap for assessing, improving, testing and preparing **OpenDEX** — a peer-to-peer (P2P) decentralized exchange built in Rust — for a controlled release.

OpenDEX is a Rust workspace comprising 20+ crates organized into three development milestones, a native desktop GUI (iced), a static web portal (Leptos/WASM), and a P2P network layer (libp2p with gossipsub, Kademlia DHT, and Noise encryption). The primary trading pair is BTC–XMR atomic swaps, with a design that maintains a non-custodial, decentralized architecture rated **GREEN** under the project's regulatory compliance framework.

The roadmap covers infrastructure, testnet deployment, core functionality, P2P communication, desktop interfaces, security auditing, regulatory compliance, performance, release management and operational readiness.

All proposed changes should be recorded, prioritised, assigned an owner, tested against defined acceptance criteria and approved before release.

---

## 2. Objectives

The roadmap aims to:

- Confirm the current technical status of the OpenDEX Rust codebase, its 20+ crates, and their inter-dependencies.
- Validate the three-milestone development plan (BTC–XMR atomic swap foundation; persistence, watchers, recovery; P2P transport, peer discovery, messaging).
- Build and maintain reliable development, testnet and staging environments on Ubuntu VPS.
- Validate core exchange, order matching, trade lifecycle, settlement and P2P gossipsub functionality.
- Improve the reliability and usability of the desktop application (iced) and web portal (Leptos/WASM).
- Identify and remediate remaining security vulnerabilities; prepare for external security audit.
- Maintain and extend the regulatory compliance analysis for EU MiCA and UK FCA regimes.
- Establish repeatable CI/CD, testing, deployment, rollback and release procedures.
- Define measurable acceptance criteria for each workstream.
- Maintain a prioritised backlog of future enhancements and technical debt.

---

## 3. Project Architecture Overview

### 3.1 Workspace Structure

```
OpenDexRustSource/
├── Cargo.toml              # Workspace manifest (20+ crates)
├── deny.toml               # cargo-deny configuration
├── crates/
│   ├── common/             # Shared domain types (Asset, Market, Side)
│   ├── protocol/           # Message types: Order, Trade, TradeState, Message
│   ├── p2p-node/           # P2P node: OrderBook, TradeBook, BalanceBook, Exchange
│   │
│   │   # Milestone 1: BTC-XMR atomic swap foundation
│   ├── opendex-types/
│   ├── opendex-errors/
│   ├── opendex-protocol-types/
│   ├── opendex-protocol/
│   ├── opendex-btc-xmr/
│   ├── opendex-bitcoin/
│   ├── opendex-monero/
│   ├── opendex-swap/
│   │
│   │   # Milestone 2: Persistence, watchers, recovery
│   ├── opendex-events/
│   ├── opendex-storage/
│   ├── opendex-watchers/
│   ├── opendex-recovery/
│   │
│   │   # Milestone 3: P2P transport, peer discovery, messaging
│   ├── opendex-transport/
│   ├── opendex-chain/
│   │
│   │   # Regulatory compliance
│   ├── opendex-dao/                # DAO governance interface
│   ├── opendex-fee-treasury/       # Fee & treasury subsystem
│   │
│   │   # User interfaces
│   ├── web-frontend/               # Leptos/WASM static portal
│   └── desktop-gui/                # iced native desktop app
│
├── fuzz/                   # cargo-fuzz targets (7 fuzz targets)
├── tests/                  # Integration tests
├── audit/                  # Security audit, compliance, bug log
├── docs/
│   ├── deployment/         # Web portal deployment scripts
│   └── testnet/            # Testnet deployment scripts and test results
└── book.iced.rs            # iced GUI book
```

### 3.2 Key Components

| Component | Technology | Description |
|-----------|------------|-------------|
| **P2P Node** | libp2p 0.57 | OrderBook, TradeBook, BalanceBook, Exchange, gossipsub, Kademlia DHT |
| **Desktop GUI** | iced 0.14 | Native desktop app (Linux, macOS, Windows); thin client of local P2P node |
| **Web Portal** | Leptos/WASM | Static informational site with download links and source browser |
| **Testnet** | Ubuntu 24.04 VPS | 3 P2P nodes (1 bootstrap + 2 test), Monero stagenet, Bitcoin testnet |
| **Security** | cargo-deny, Miri, cargo-fuzz | Dependency audit, UB detection, fuzzing (7 targets) |
| **Regulatory** | MiCA, UK FCA | Comprehensive compliance analysis in `audit/regulatory_compliance/` |

### 3.3 Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Rust | 1.88+ | Minimum toolchain version |
| libp2p | 0.57 | P2P networking (TCP, Noise, yamux, Kademlia, gossipsub, identify) |
| iced | 0.14 | Desktop GUI framework |
| tokio | 1 | Async runtime |
| tracing | 0.1 | Structured logging |
| serde/serde_json | — | Serialization |

---

## 4. Current Status Summary

### 4.1 Security Audit

| Area | Status |
|------|--------|
| Code-level audit | 🟢 Complete — all High/Medium/Low findings resolved |
| cargo-audit (dependency scan) | 🟢 0 vulnerabilities, 3 low-risk warnings |
| cargo-deny | 🟢 Configured, all checks pass |
| Miri (UB detection) | 🟢 146 tests, zero undefined behavior |
| Fuzzing | 🟢 7 targets initialized, infrastructure documented |
| External security audit | ⏳ Pending |

### 4.2 Regulatory Compliance

| Area | Status |
|------|--------|
| EU MiCA analysis | 🟢 Complete |
| UK FCA analysis | 🟢 Complete |
| Architecture regulatory risk assessment | 🟢 Complete |
| Governance boundaries (three-way domain separation) | 🟢 Documented |
| DAO interface design | 🟢 Sketch complete, crate initialized |
| Fee & treasury subsystem | 🟢 Crate initialized |
| Desktop GUI regulatory rating | 🟢 GREEN |

### 4.3 Bug & Issue Tracker

| Category | Status |
|----------|--------|
| Critical issues (ARCH-001, ARCH-004) | 🟢 Resolved |
| High-priority bugs (BUG-001, AUDIT-H01, AUDIT-H02) | 🟢 Resolved |
| Deferred items (BUG-002, etc.) | 🟡 Documented |
| Open issues | See `audit/bug_log.md` |

### 4.4 Testnet Infrastructure

| Component | Status |
|-----------|--------|
| VPS setup scripts | 🟢 Complete |
| Wallet daemon setup (Monero stagenet, Bitcoin testnet) | 🟢 Complete |
| Web frontend deployment (Nginx + HTTPS) | 🟢 Complete |
| P2P node deployment (systemd services) | 🟢 Complete |
| One-command deployment script | 🟢 Complete |
| Health check and e2e test scripts | 🟢 Complete |

### 4.5 CI/CD

| Area | Status |
|------|--------|
| GitHub Actions / CI pipeline | 🔴 Not yet configured |
| Automated test execution | Partially covered (workspace tests) |
| Automated linting (clippy) | Manual |
| Automated dependency checks | Manual (cargo-deny, cargo-audit) |
| Release artefact management | Manual (build scripts) |

---

## 5. Scope and Workstreams

### WS01 — Project Assessment and Architecture

**Current status:** Substantially complete.

**Activities**
- Review the existing repositories, branches, releases and open issues.
- Establish the current project maintenance and contribution arrangements.
- Review the architecture and dependencies between the P2P node, desktop GUI, web portal and supporting services.
- Confirm supported operating systems (Linux, macOS, Windows), protocols (libp2p) and integrations (Monero, Bitcoin).
- Identify incomplete functionality, known defects and technical debt.
- Establish the baseline against which future changes will be measured.

**Deliverables**
- Current-state assessment. (🟢 Partially complete — see Section 4)
- Architecture and dependency diagrams. (🟢 Present in this document and `audit/archive/admp_v5.md`)
- Known issues and technical debt register. (🟢 `audit/bug_log.md`)
- Prioritised change backlog.
- Agreed scope and release objectives.

**Exit criteria:** The project team has an agreed understanding of the current system, its limitations and its delivery priorities.

---

### WS02 — Development, Testnet and Staging

**Current status:** Testnet deployment infrastructure is complete; CI/CD pipeline needs to be established.

**Activities**
- Establish repeatable development and testnet environments using existing scripts in `docs/testnet/`.
- Confirm supported network configurations: Monero stagenet, Bitcoin testnet.
- Validate node startup, synchronisation and connectivity across 3-node testnet.
- Test transaction processing and settlement flows on testnet.
- Establish test data creation, reset and cleanup procedures.
- Separate development, testnet, staging and production configurations.
- Document environment setup and recovery procedures.
- **Set up CI/CD pipeline** (GitHub Actions or equivalent) for automated builds, tests, clippy, cargo-deny, and cargo-audit.

**Deliverables**
- Environment configuration documentation. (🟢 `docs/testnet/README.md`)
- Testnet deployment guide. (🟢 `docs/testnet/deploy-testnet.sh`)
- Network and transaction test suite. (🟢 `docs/testnet/test-results/`)
- Environment health checks. (🟢 `docs/testnet/test-results/verification/`)
- Testnet defect and results register.
- **CI/CD pipeline configuration.**

**Exit criteria:** The team can reliably deploy the test environment, execute representative workflows and reproduce reported defects. CI pipeline runs on every PR.

---

### WS03 — VPS and Infrastructure

**Current status:** VPS deployment scripts are complete; monitoring and alerting need to be added.

**Activities**
- Assess hosting requirements, resource utilisation and network topology.
- Configure server provisioning, operating-system updates and access controls.
- Apply firewall rules and restrict administrative interfaces.
- Configure DNS and TLS where applicable (Let's Encrypt via Certbot).
- **Implement monitoring, centralised logging and alerting.**
- Establish backup and restoration procedures.
- Test server restart, service recovery and infrastructure failure scenarios.
- Document deployment and disaster recovery procedures.

**Deliverables**
- VPS architecture and configuration baseline. (🟢 `docs/testnet/README.md`)
- Server hardening checklist.
- **Monitoring and alerting configuration.**
- **Backup and recovery runbook.**
- Infrastructure performance and resilience results.

**Exit criteria:** The environment meets agreed security, availability, monitoring and recovery requirements.

---

### WS04 — Core OpenDEX Functionality

**Current status:** Core P2P node functionality is implemented and audited; integration testing on testnet needs to be completed.

**Activities**
- Validate P2P node startup and configuration.
- Test peer discovery and connectivity via gossipsub and Kademlia DHT.
- Verify order-book creation, updates and synchronisation.
- Validate order matching and cancellation behaviour.
- Test supported trade and swap protocols (BTC–XMR atomic swaps).
- Validate transaction settlement and failure handling.
- Verify fee handling, balances and transaction status reporting.
- Test invalid inputs, interrupted operations and recovery scenarios.
- Validate persistence layer (opendex-storage) and recovery (opendex-recovery).
- Test watcher functionality (opendex-watchers) for on-chain events.

**Deliverables**
- Functional requirements and test cases.
- Core functionality test report.
- Integration test results.
- Defect register and remediation plan.

**Exit criteria:** All critical user and system workflows pass their acceptance tests, with no unresolved release-blocking defects.

---

### WS05 — Web Portal

**Current status:** Web portal is implemented and deployable.

**Activities**
- Test navigation, layout and interface consistency.
- Test supported browsers and responsive layouts.
- Verify source code browser functionality.
- Verify legal and compliance content is current.
- Perform regression testing after interface changes.
- Validate deployment via `docs/deployment/deploy.sh`.

**Deliverables**
- Web portal test plan.
- Browser compatibility results.
- Usability findings.
- Defect and enhancement backlog.

**Exit criteria:** All critical web portal workflows operate correctly and content is current.

---

### WS06 — Desktop Application

**Current status:** Desktop GUI is implemented with iced 0.14; cross-platform testing and release packaging need attention.

**Activities**
- Test installation, startup, shutdown and removal on Linux, macOS, and Windows.
- Verify compatibility with supported operating systems.
- Validate local environment detection and connection to OpenDEX P2P node.
- Test configuration, logs and diagnostic information.
- Verify upgrades, clean installations and rollback procedures.
- Test connectivity loss, application crashes and recovery.
- Review application permissions, data storage and secret handling.
- Validate release packaging and software integrity.
- Test optional Monero and Bitcoin wallet RPC integration.

**Deliverables**
- Desktop compatibility matrix.
- Installation and upgrade test report.
- Desktop security assessment.
- Release packaging checklist.

**Exit criteria:** The application installs and operates reliably on all supported platforms, with documented recovery procedures.

---

### WS07 — Peer-to-Peer Testing

**Current status:** P2P protocol is implemented with libp2p; multi-node and adverse-network testing needs to be completed.

**Activities**
- Validate peer discovery, connection establishment and disconnection.
- Test message formats and protocol compatibility.
- Verify order and trade message propagation via gossipsub.
- Test multiple peers and independent node instances.
- Simulate network latency, packet loss and intermittent connectivity.
- Validate reconnection and synchronisation after outages.
- Test malformed messages, invalid orders and untrusted peers.
- Assess resistance to peer flooding, spam and resource exhaustion.
- Test supported privacy and network-routing configurations.
- Validate interoperability between supported implementations.

**Deliverables**
- P2P test scenarios and results.
- Multi-node integration report.
- Network resilience report.
- Protocol compatibility findings.
- P2P defect register.

**Exit criteria:** Supported peer interactions remain correct and recover safely under agreed normal and adverse network conditions.

---

### WS08 — Security Auditing

**Current status:** Internal security audit is complete; external audit is pending.

**Activities**
- Conduct threat modelling and attack-surface analysis. (🟢 Partially complete — `audit/security.md`)
- Perform source-code and dependency security reviews. (🟢 Complete)
- Review authentication, authorisation and API exposure. (🟢 Complete)
- Assess cryptographic operations, private-key handling and transaction signing.
- Review container, VPS and operating-system security.
- Test web and desktop application security.
- Assess P2P protocol abuse and denial-of-service risks.
- **Conduct independent penetration testing.**
- Remediate findings and retest fixes.
- Establish a vulnerability disclosure and incident response process.

**Deliverables**
- Threat model and security risk register. (🟢 `audit/security.md`)
- Security audit report. (🟢 `audit/audit.md`)
- Remediation plan. (🟢 All code-level findings resolved)
- **External audit report.**
- Release security sign-off.

**Exit criteria:** No unresolved critical or high-risk findings remain unless an explicitly authorised exception has been documented and accepted.

---

### WS09 — Regulatory Compliance and Legal Review

**Current status:** Comprehensive regulatory analysis is complete; legal sign-off is pending.

**Activities**
- Identify relevant operating jurisdictions and target users. (🟢 Complete)
- Document the proposed operating model, including non-custody, exchange and service-provider arrangements. (🟢 Complete)
- Determine which financial-services, AML/CFT, sanctions and consumer-protection obligations may apply. (🟢 Complete)
- Assess privacy, personal-data processing and retention requirements. (🟢 Complete)
- Review terms of service, risk disclosures and complaints processes. (🟢 `crates/web-frontend/src/legal.rs`)
- Establish record-keeping and audit-trail requirements.
- Document responsibility for ongoing compliance monitoring.
- **Obtain qualified legal or compliance advice on unresolved questions.**

**Deliverables**
- Regulatory applicability assessment. (🟢 `audit/regulatory_compliance/regulatory_compliance.md`)
- Compliance obligations register. (🟢 Present)
- Privacy and data-handling assessment. (🟢 Present)
- Required policy and disclosure documents. (🟢 Present in web portal)
- **Compliance review and sign-off record.**

**Exit criteria:** Applicable obligations have been identified, required controls are implemented, and outstanding legal or regulatory blockers have been resolved.

**Note:** A decentralised or non-custodial design does not automatically remove legal obligations. Applicability must be assessed against the actual product, operating model and jurisdictions. The project's regulatory analysis covers EU MiCA (Recital 22, ESMA guidance) and UK FCA (perimeter guidance, CP26/13).

---

### WS10 — API, Integration and Data Integrity

**Current status:** Core integration between P2P node and GUI is implemented; formal API testing needs to be completed.

**Activities**
- Validate API contracts, request handling and error responses.
- Test authentication and rate limiting where applicable.
- Verify integration between the P2P node, desktop GUI and web portal.
- Test database or local state consistency (opendex-storage).
- Validate configuration changes and migrations.
- Test interrupted transactions and recovery from inconsistent states (opendex-recovery).
- Check dependency compatibility and version management.

**Deliverables**
- API test suite.
- Integration test report.
- Data integrity test results.
- Compatibility and dependency register. (🟢 `deny.toml`)

**Exit criteria:** Components communicate reliably, failures are handled safely, and state remains consistent across supported workflows.

---

### WS11 — Performance, Scalability and Resilience

**Current status:** Not yet started.

**Activities**
- Establish baseline resource consumption for P2P node and desktop GUI.
- Measure response times and transaction processing performance.
- Test increasing numbers of peers and concurrent operations.
- Assess CPU, memory, disk and network utilisation.
- Test extended operation and resource leaks.
- Simulate service outages and degraded dependencies.
- Define capacity thresholds and scaling requirements.

**Deliverables**
- Performance baseline.
- Load and stress test report.
- Capacity assessment.
- Resilience findings and improvement plan.

**Exit criteria:** The system meets agreed performance targets and recovers predictably from tested failure conditions.

---

### WS12 — CI/CD and Change Control

**Current status:** Not yet started. This is a high-priority gap.

**Activities**
- Define the change request and approval workflow.
- Establish branch management and code review requirements.
- **Set up GitHub Actions (or equivalent) CI pipeline** with:
  - Automated `cargo build` and `cargo test` on every PR
  - Automated `cargo clippy` linting
  - Automated `cargo-deny` checks
  - Automated `cargo-audit` dependency scans
  - Fuzzing smoke tests
- Introduce dependency and security checks into build pipelines.
- Maintain versioned release artefacts.
- Require documented deployment and rollback plans.
- Separate development, test and production permissions.
- Record release approvals and implementation outcomes.

**Deliverables**
- Change management procedure.
- **Automated build and test pipeline (GitHub Actions).**
- Release checklist.
- Deployment and rollback runbooks.
- Version and change register.

**Exit criteria:** Changes are traceable from request through approval, implementation, testing and release. Every PR passes CI before merge.

---

### WS13 — Quality Assurance and User Acceptance Testing

**Current status:** Workspace tests exist (289+ tests); formal QA plan and UAT need to be established.

**Activities**
- Establish a master test plan covering all crates.
- Define functional, regression, integration and acceptance test suites.
- Record expected results and actual outcomes.
- Classify and prioritise defects.
- Conduct user acceptance testing with representative workflows.
- Retest resolved defects.
- Obtain formal acceptance from the designated owner.

**Deliverables**
- Master test plan.
- Test execution evidence.
- Defect and retest register. (🟢 `audit/bug_log.md`)
- User acceptance report.

**Exit criteria:** Mandatory tests pass, release-blocking defects are resolved, and the authorised owner accepts the results.

---

### WS14 — Monitoring and Operational Readiness

**Current status:** Not yet started.

**Activities**
- Define operational health indicators for P2P nodes and services.
- Configure alerts for service failures and abnormal behaviour.
- Establish incident severity and escalation procedures.
- Test backup restoration and recovery.
- Document routine maintenance and upgrades.
- Define support ownership and incident communications.
- Establish post-release monitoring and review arrangements.

**Deliverables**
- Monitoring dashboard and alert rules.
- Operational runbooks.
- Incident response procedure.
- Support and escalation matrix.
- Recovery test evidence.

**Exit criteria:** The responsible team can monitor, support and recover the system using documented procedures.

---

### WS15 — Production Readiness and Launch

**Current status:** Not yet started.

**Activities**
- Confirm release scope and version.
- Review test, security, compliance and operational evidence.
- Confirm that all required approvals have been obtained.
- Verify deployment and rollback procedures.
- Complete final backup and recovery checks.
- Confirm support coverage and incident escalation.
- Deploy through the approved release process.
- Monitor initial operation and resolve emerging issues.
- Conduct a post-implementation review.

**Deliverables**
- Production readiness checklist.
- Go/no-go decision record.
- Approved release and rollback plan.
- Deployment report.
- Post-implementation review and follow-up backlog.

**Exit criteria:** The release has been authorised, deployment outcomes have been verified and ongoing ownership has been established.

---

## 6. Indicative Delivery Timeline

The following is a planning baseline, not a committed delivery schedule. Actual timing depends on team capacity, technical dependencies and audit findings.

**Note on current status:** WS01 (Assessment), WS08 (Security — internal audit), WS09 (Regulatory — analysis), and WS02 (Testnet — deployment scripts) are substantially complete. The remaining work focuses on CI/CD, integration testing, external audit, and production readiness.

| Phase | Indicative timing | Primary outcome |
|-------|-------------------|-----------------|
| Phase 1: CI/CD and test infrastructure | Weeks 1–3 | Automated pipeline, regression suite |
| Phase 2: Core functionality validation | Weeks 2–6 | Functional and integration test coverage |
| Phase 3: P2P and resilience testing | Weeks 4–8 | Network reliability evidence |
| Phase 4: Desktop GUI hardening | Weeks 4–7 | Cross-platform compatibility, release packaging |
| Phase 5: External security audit | Weeks 6–10+ | Independent audit report, remediation |
| Phase 6: Performance and scalability | Weeks 7–9 | Performance baseline, capacity assessment |
| Phase 7: Operational readiness | Weeks 8–10 | Monitoring, alerting, runbooks |
| Phase 8: Acceptance and release readiness | Weeks 9–12+ | Formal go/no-go decision |
| Phase 9: Deployment and review | After release approval | Controlled deployment and monitoring |

Security, compliance and operational planning should continue throughout delivery. Production deployment must not proceed solely because the indicative timeline has elapsed.

---

## 7. Change Request and Approval Process

Every proposed change should follow this workflow:

1. **Request:** Record the change ID, reason, scope, expected benefit and requester.
2. **Assess:** Identify technical impact, security risks, compliance implications and dependencies.
3. **Prioritise:** Assign priority based on risk, value, urgency and effort.
4. **Approve:** Obtain the required authorisation before implementation.
5. **Implement:** Develop the change in the appropriate branch and environment.
6. **Test:** Execute relevant functional, regression, security and acceptance tests. CI pipeline must pass.
7. **Release:** Obtain release authorisation and deploy using the approved procedure.
8. **Review:** Confirm the outcome, document issues and close the change record.

Emergency changes must still be recorded, appropriately authorised and reviewed retrospectively under the agreed emergency procedure.

---

## 8. Roles and Responsibilities

| Role | Primary responsibility |
|------|----------------------|
| Project owner | Scope, priorities, resources and delivery decisions |
| Technical lead | Architecture, technical decisions and implementation quality |
| Infrastructure lead | VPS, environments, monitoring and recovery |
| Developers | Implementation, code review and defect resolution |
| QA / test lead | Test planning, execution and evidence |
| Security lead / auditor | Security assessment, remediation validation and sign-off |
| Legal / compliance lead | Regulatory applicability and compliance review |
| Release manager | Release coordination, approvals and rollback readiness |
| Business / user representative | Requirements and user acceptance |

One person may cover multiple roles in a small team, provided responsibilities and approval authority remain clear.

---

## 9. Risk and Dependency Register

| Risk | Mitigation |
|------|-----------|
| Unclear maintenance status or unsupported dependencies | `cargo-deny` configured; `cargo-audit` clean; validate supported versions |
| Incomplete or unreliable testnet | Repeatable deployment scripts exist; add automated health checks to CI |
| Critical security vulnerabilities | Internal audit complete; external audit pending; fuzzing infrastructure active |
| P2P interoperability failures | Test multiple nodes on testnet; fuzz protocol deserialization |
| Transaction or settlement errors | 289+ workspace tests; per-order reservation tracking; idempotent settlement |
| VPS outages or data loss | Implement monitoring, backups and tested recovery procedures |
| Unresolved regulatory questions | Comprehensive MiCA/FCA analysis complete; obtain qualified legal advice |
| Regression introduced by changes | Automate regression testing in CI; require release evidence |
| Uncontrolled production changes | Enforce approval, deployment and rollback procedures |
| Insufficient support after release | Assign operational ownership and establish incident response procedures |
| **libp2p / iced upstream changes** | Pin versions; monitor upstream; test after upgrades |
| **Monero / Bitcoin RPC instability** | Test with stagenet/testnet; implement retry and fallback logic |

---

## 10. Reporting and Key Performance Indicators

Progress should be reviewed weekly using a common dashboard.

Recommended measures include:

- Percentage of roadmap tasks completed.
- Number of open defects by severity (tracked in `audit/bug_log.md`).
- Percentage of mandatory tests passed (workspace test count: 289+).
- Number of unresolved security findings by severity.
- Percentage of changes with complete approval and test evidence.
- Testnet and service availability against agreed targets.
- Number of outstanding release blockers.
- Backup and recovery test completion.
- Compliance actions completed and outstanding.
- Release success rate and post-release incident count.
- **CI pipeline pass rate.**
- **Fuzzing corpus growth and crash count.**

Targets should be agreed after the initial assessment rather than invented before a baseline exists.

---

## 11. Definition of Done

A change is complete only when:

- Requirements and acceptance criteria have been documented.
- Required reviews and approvals have been obtained.
- Relevant automated and manual tests have passed.
- CI pipeline passes (build, test, clippy, cargo-deny).
- Security and compliance impacts have been assessed.
- Documentation and configuration records have been updated.
- Known defects and accepted risks have been recorded.
- Deployment and rollback arrangements are ready.
- The authorised owner has accepted the outcome.

---

## 12. Governance and Review

The roadmap should be reviewed weekly during active development and formally at each major phase gate.

Changes to scope, priority, milestones or release criteria must be documented. Material security, compliance or operational risks must be escalated to the designated decision-maker.

The roadmap should remain a living document, updated as new defects, requirements, dependencies and technical findings emerge.

---

## 13. Immediate Next Steps

1. Identify the technical lead and workstream owners.
2. **Set up CI/CD pipeline** (GitHub Actions) with automated build, test, clippy, cargo-deny, and cargo-audit.
3. Complete the prioritised change backlog from `audit/bug_log.md` deferred items.
4. Set up or validate the testnet and staging environments using existing scripts.
5. Create the master change register and defect backlog.
6. Define mandatory testing and security gates.
7. **Commission external security audit.**
8. Agree priorities, resource availability and realistic target dates.
9. Review and approve the baseline roadmap before committing to release milestones.

---

## Approval

| Role | Name | Signature | Date |
|------|------|-----------|------|
| Project owner | | | |
| Technical lead | | | |
| Security / compliance reviewer | | | |

---

## Appendix A: Reference Documents

| Document | Location |
|----------|----------|
| Security Audit | `audit/audit.md` |
| Security Log | `audit/security.md` |
| Bug & Issue Tracker | `audit/bug_log.md` |
| Security Audit Checklist | `audit/audit_checklist.md` |
| Escrow Design | `audit/escrow_design.md` |
| Regulatory Compliance (Master) | `audit/regulatory_compliance/regulatory_compliance.md` |
| Architecture (ADMP v5, archived) | `audit/archive/admp_v5.md` |
| Development Log | `audit/dev_log.md` |
| Fuzzing Process | `fuzz/FUZZ.md` |
| Fuzzing Infrastructure | `fuzz/INFRASTRUCTURE.md` |
| Testnet Deployment | `docs/testnet/README.md` |
| Web Portal Deployment | `docs/deployment/DEPLOY.md` |
| Desktop GUI | `crates/desktop-gui/README.md` |
| Web Portal | `crates/web-frontend/README.md` |
| GUI Development Plan | `.junie/plans/opendex-gui-development-plan.md` |
| Dependency Policy | `deny.toml` |

---

## Appendix B: Milestone Definitions

### Milestone 1: BTC–XMR Atomic Swap Foundation

Crates: `opendex-types`, `opendex-errors`, `opendex-protocol-types`, `opendex-protocol`, `opendex-btc-xmr`, `opendex-bitcoin`, `opendex-monero`, `opendex-swap`.

Foundation for BTC–XMR atomic swap protocol, including types, error handling, protocol messages, and chain-specific integrations.

### Milestone 2: Persistence, Watchers, Recovery

Crates: `opendex-events`, `opendex-storage`, `opendex-watchers`, `opendex-recovery`.

Persistent state management, on-chain event watching, and recovery from partial or interrupted operations.

### Milestone 3: P2P Transport, Peer Discovery, Messaging

Crates: `opendex-transport`, `opendex-chain`.

Advanced P2P transport layer, peer discovery mechanisms, and inter-node messaging protocols.

### Regulatory Compliance Crates

Crates: `opendex-dao`, `opendex-fee-treasury`.

DAO governance interface and fee/treasury subsystem, designed to maintain the project's non-custodial, decentralized regulatory posture.
