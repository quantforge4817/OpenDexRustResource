# OpenDex Audit Scope

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Proposed scope for independent security assessment
**Scope:** Components to assess, exclusions, priority questions, and expected deliverables

---

## Table of Contents

1. [Audit Objectives](#1-audit-objectives)
2. [In-Scope Components](#2-in-scope-components)
3. [Out-of-Scope Components](#3-out-of-scope-components)
4. [Priority Questions](#4-priority-questions)
5. [Expected Deliverables](#5-expected-deliverables)
6. [Reviewer Requirements](#6-reviewer-requirements)
7. [Engagement Model](#7-engagement-model)
8. [Timeline](#8-timeline)

---

## 1. Audit Objectives

The primary objective is to obtain an independent assessment of OpenDex's cryptographic assumptions, cross-chain transaction logic, recovery mechanisms, and implementation security before production deployment.

### 1.1 Specific Goals

1. **Protocol design assessment** — Verify the BTC-XMR atomic swap protocol design and identify major security assumptions
2. **Implementation review** — Examine the Rust implementation for correctness, security, and robustness
3. **Adversarial testing** — Test selected adversarial and failure scenarios
4. **Fiat payment review** — Assess the fiat-payment coordination and application security
5. **Remediation verification** — Verify critical fixes within the agreed scope

### 1.2 What This Audit Is NOT

- Not a guarantee of security — an independent review reduces uncertainty but does not prove every vulnerability is eliminated
- Not a smart contract audit — this is a standalone Rust application, not a blockchain smart contract
- Not a regulatory compliance review — legal and regulatory assessment is separate

---

## 2. In-Scope Components

### 2.1 Critical Priority

| Component | Crate | Description |
|-----------|-------|-------------|
| DLEQ proof integration | `opendex-btc-xmr` | Cross-curve DLEQ proof generation and verification |
| Atomic swap state machine | `opendex-protocol-types` | 10-state swap lifecycle |
| Swap engine | `opendex-swap` | Generic swap coordination |
| Recovery engine | `opendex-recovery` | Deterministic recovery decisions |
| Bitcoin chain interface | `opendex-bitcoin` | Transaction construction and broadcasting |
| Monero chain interface | `opendex-monero` | Transaction construction and broadcasting |

### 2.2 High Priority

| Component | Crate | Description |
|-----------|-------|-------------|
| Fiat trade state machine | `fiat::trade` | 16-state fiat trade lifecycle |
| Settlement engine | `fiat::settlement_engine` | Trade orchestration |
| Bitcoin multisig escrow | `fiat::multisig` | 2-of-2 P2WSH co-signing |
| Payment adapters | `fiat::adapter` | Payment detection and verification |
| Security bonds | `fiat::bond` | Bond calculation and management |
| Dispute resolution | `fiat::dispute` | Evidence collection and resolution |
| Transport layer | `opendex-transport` | Message delivery and replay protection |
| Storage | `opendex-storage` | Persistent state with atomic writes |
| Event sourcing | `opendex-events` | Immutable event log |

### 2.3 Medium Priority

| Component | Crate | Description |
|-----------|-------|-------------|
| P2P node | `p2p-node` | libp2p integration, network behavior |
| User authentication | `user` | Argon2id passwords, HMAC tokens |
| Chain watchers | `opendex-watchers` | Blockchain observation |
| Protocol registry | `opendex-protocol` | Version negotiation |
| Fee calculation | `opendex-fee-treasury` | Fee and treasury management |
| Type system | `opendex-types` | Domain types and validation |
| Error handling | `opendex-errors` | Unified error hierarchy |

### 2.4 Low Priority

| Component | Crate | Description |
|-----------|-------|-------------|
| DAO governance | `opendex-dao` | Voting and governance |
| Common utilities | `common` | Shared helpers |
| Legacy protocol | `protocol` | Legacy P2P message types |

---

## 3. Out-of-Scope Components

### 3.1 Explicitly Excluded

| Component | Reason |
|-----------|--------|
| `desktop-gui` | GUI application; not security-critical |
| `web-frontend` | Web interface; not security-critical |
| GUI transitive dependencies | `paste`, `ttf-parser`, `lru` — not in financial code path |
| Third-party cryptographic libraries | `secp256k1`, `sha2`, `argon2` — assumed correct |
| libp2p core library | External dependency; assumed correct |
| Regulatory compliance | Legal assessment is separate |
| Performance benchmarking | Not a security concern |
| User interface design | Not a security concern |

### 3.2 Conditional Inclusion

| Component | Condition for Inclusion |
|-----------|------------------------|
| Desktop GUI | If reviewer has capacity after core components |
| Web frontend | If reviewer has capacity after core components |
| DAO governance | If reviewer identifies relevant security concerns |

---

## 4. Priority Questions

### 4.1 Cryptographic Protocol

1. **Is the DLEQ proof construction correct?** — Once Eigenwallet integration is complete, is the cross-curve proof sound?
2. **Are the cryptographic assumptions valid?** — Do the secp256k1/Ed25519 cross-curve key derivation assumptions hold?
3. **Are adaptor signatures correctly implemented?** — Does the adaptor signature construction provide atomic redemption?
4. **Is the secret revelation protocol safe?** — Can either party gain an unfair advantage through the secret revelation process?

### 4.2 Transaction Logic

5. **Are Bitcoin transactions correctly constructed?** — Are funding, redemption, and refund transactions valid?
6. **Are Monero transactions correctly constructed?** — Are view-tagged outputs properly encrypted?
7. **Are timeout mechanisms adequate?** — Can a malicious party exploit timeout windows?
8. **Is the 2-of-2 multisig escrow secure?** — Can funds be released without both parties' consent?

### 4.3 State Machine

9. **Are all state transitions valid?** — Can the state machine be forced into an inconsistent state?
10. **Are recovery paths complete?** — Can funds always be recovered from any state?
11. **Are cancellation paths complete?** — Can a trade always be cancelled before completion?
12. **Are dispute paths adequate?** — Can disputes be resolved fairly?

### 4.4 Fiat Payment

13. **Is the payment coordination secure?** — Can a malicious party exploit the fiat payment flow?
14. **Are payment instructions properly bound?** — Can bank details be substituted after trade acceptance?
15. **Is the bond system effective?** — Do bonds adequately deter bad behavior?
16. **Are payment reversals handled?** — What happens if a fiat payment is reversed after crypto release?

### 4.5 Implementation

17. **Are there memory safety issues?** — Beyond the 2 known `unsafe` blocks, are there any memory safety concerns?
18. **Is error handling adequate?** — Are errors properly propagated and handled?
19. **Is the storage layer robust?** — Can storage corruption lead to fund loss?
20. **Are there race conditions?** — Can concurrent operations lead to inconsistent state?

---

## 5. Expected Deliverables

### 5.1 Required Deliverables

| Deliverable | Description |
|-------------|-------------|
| **Written security assessment** | Comprehensive report of findings with severity ratings |
| **Reproducible evidence** | Proof-of-concept exploits or test cases for identified vulnerabilities |
| **Prioritized remediation plan** | Ordered list of fixes by severity and effort |
| **Record of findings** | Complete list of resolved and unresolved findings |
| **Final report** | Document identifying reviewed commit, scope, methodology, and exclusions |

### 5.2 Report Format

The report should follow this structure:

1. **Executive Summary** — High-level findings and recommendations
2. **Reviewed Components** — List of assessed code and protocol components
3. **Methodology** — Review approach and tools used
4. **Findings** — Detailed findings with:
   - Unique reference (e.g., AUDIT-C01)
   - Severity rating (Critical/High/Medium/Low/Informational)
   - Description and affected component
   - Reproduction steps or proof-of-concept
   - Impact assessment
   - Recommended remediation
5. **Positive Findings** — Security controls that are well-implemented
6. **Limitations** — What was not covered or could not be verified
7. **Conclusion** — Overall assessment and launch recommendation

### 5.3 Severity Ratings

| Severity | Definition | Example |
|----------|-----------|---------|
| **Critical** | Direct fund loss or complete protocol bypass | DLEQ proof forgery enabling fund theft |
| **High** | Significant fund risk or major security weakness | Missing cancellation path locking funds |
| **Medium** | Moderate risk or partial security weakness | Missing timeout allowing delayed exploitation |
| **Low** | Minor risk or best-practice improvement | Missing input validation on non-critical field |
| **Informational** | No security impact; informational note | Suggested improvement or documentation gap |

---

## 6. Reviewer Requirements

### 6.1 Required Expertise

| Role | Expertise |
|------|-----------|
| **Atomic-swap protocol specialist** | Bitcoin–Monero swaps, cross-chain transaction conditions, timeout mechanisms, refund paths, adversarial protocol behaviour |
| **Cryptography specialist** | DLEQ proofs, cross-curve constructions, adaptor signatures, zero-knowledge proofs, elliptic curve cryptography |
| **Rust security engineer** | Implementation correctness, unsafe code, dependencies, error handling, state management, transaction-processing logic |
| **Application security reviewer** | Fiat-payment coordination, participant authentication, messaging integrity, privacy, operational failure scenarios |

### 6.2 Preferred Experience

- Prior audit of a cross-chain atomic swap protocol
- Experience with Bitcoin transaction construction and multisig
- Experience with Monero transaction construction and privacy features
- Prior Rust security audits of financial applications
- Experience with libp2p or P2P protocol security

### 6.3 Minimum Qualifications

- Demonstrable experience with cryptographic protocol analysis
- Published work or verifiable contributions to cryptocurrency security
- References from prior audit engagements

---

## 7. Engagement Model

### 7.1 Proposed Stages

| Stage | Duration | Deliverable |
|-------|----------|-------------|
| **Stage 1: Protocol design assessment** | 1–2 weeks | Identification of major security assumptions and design flaws |
| **Stage 2: Implementation review** | 2–4 weeks | Code-level findings and vulnerability analysis |
| **Stage 3: Adversarial testing** | 1–2 weeks | Proof-of-concept exploits and edge-case findings |
| **Stage 4: Remediation verification** | 1–2 weeks | Verification of critical fixes |
| **Stage 5: Final reporting** | 1 week | Complete written report |

### 7.2 Communication

| Channel | Purpose |
|---------|---------|
| **Weekly updates** | Progress report and preliminary findings |
| **Critical finding notification** | Immediate notification of Critical/High findings |
| **Remediation discussion** | Technical discussion of proposed fixes |
| **Final review meeting** | Walkthrough of final report |

### 7.3 Confidentiality

- All findings are confidential until mutually agreed for publication
- Critical findings are reported privately before any public disclosure
- The final report is shared with the project team before publication
- Responsible disclosure timeline: 90 days from finding to public disclosure

---

## 8. Timeline

### 8.1 Proposed Schedule

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Protocol design assessment | 2 weeks | Week 1 | Week 2 |
| Implementation review | 4 weeks | Week 3 | Week 6 |
| Adversarial testing | 2 weeks | Week 7 | Week 8 |
| Remediation (project team) | 2–4 weeks | Week 9 | Week 12 |
| Remediation verification | 2 weeks | Week 13 | Week 14 |
| Final reporting | 1 week | Week 15 | Week 15 |

**Total estimated duration:** 15 weeks (approximately 4 months)

### 8.2 Budget

The budget will be determined by reviewer estimates. A staged engagement allows for budget adjustment after Stage 1 if major issues are found.

---

## Appendix A: Review Package Contents

The following documents are available for the reviewer:

| Document | Purpose |
|----------|---------|
| `protocol.md` | Complete protocol specification |
| `architecture.md` | System architecture and component responsibilities |
| `threat_model.md` | Threat model, attacker capabilities, and residual risks |
| `cryptography.md` | Cryptographic primitives, libraries, and assumptions |
| `testing.md` | Test coverage, fuzzing, and adversarial scenarios |
| `audit_scope.md` | This document — scope and expectations |
| `known_issues.md` | Known defects and unresolved risks |
| `reproducibility.md` | Build instructions and environment setup |
| `security.md` | Security policy, baseline, controls, and findings |

---

## Appendix B: Code Access

| Item | Details |
|------|---------|
| Repository | Private repository (access provided upon NDA) |
| Reviewed commit | `512f4cd` (tag: `v0.1.0-security-review`) |
| Branch | `main` |
| Crates | 23 Rust crates |
| Lines of code | ~32,400 lines of Rust |
| License | To be determined |
