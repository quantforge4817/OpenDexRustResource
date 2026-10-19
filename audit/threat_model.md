# OpenDex Threat Model

**Version:** 0.1.0-security-review
**Date:** 2026-10-14
**Status:** Authoritative threat model reference
**Scope:** BTC-XMR atomic swap protocol and fiat-crypto P2P trading protocol

---

## Table of Contents

1. [Threat Model Overview](#1-threat-model-overview)
2. [Trust Assumptions](#2-trust-assumptions)
3. [Attacker Capabilities](#3-attacker-capabilities)
4. [Atomic Swap Threats](#4-atomic-swap-threats)
5. [Fiat Trade Threats](#5-fiat-trade-threats)
6. [Blockchain-Specific Threats](#6-blockchain-specific-threats)
7. [Cryptographic Threats](#7-cryptographic-threats)
8. [Network and Transport Threats](#8-network-and-transport-threats)
9. [Application-Level Threats](#9-application-level-threats)
10. [Failure Scenarios](#10-failure-scenarios)
11. [Residual Risks](#11-residual-risks)
12. [Security Properties](#12-security-properties)

---

## 1. Threat Model Overview

This threat model describes the actual implementation of OpenDex, not merely the intended behaviour of the protocol. It identifies attacker capabilities, trust assumptions, security properties, failure scenarios, and residual risks.

### 1.1 System Boundaries

The system consists of:

- **Local components** — The user's application, wallet, and stored state
- **External services** — Bitcoin nodes (via Electrum RPC), Monero nodes (via daemon RPC), bank/PSP APIs
- **Untrusted participants** — Counterparties on the P2P network
- **Untrusted network** — The P2P transport layer and internet infrastructure

### 1.2 Threat Model Methodology

This model follows a STRIDE-based approach, adapted for a non-custodial cross-chain exchange:

| Category | Description |
|----------|-------------|
| **S**poofing | Impersonating another participant or service |
| **T**ampering | Modifying messages, transactions, or state |
| **R**epudiation | Denying a transaction or payment |
| **I**nformation Disclosure | Exposing secrets, keys, or sensitive data |
| **D**enial of Service | Disrupting swap progress or availability |
| **E**levation of Privilege | Gaining unauthorized access to funds or controls |

### 1.3 Key Design Constraints

- **Non-custodial:** The DEX never holds user funds. Atomic swaps use protocol-level locking; fiat trades use 2-of-2 multisig requiring both parties.
- **No central authority:** There is no operator who can freeze, seize, or reverse transactions.
- **Fiat is external:** Bank transfers settle outside the protocol. The protocol can only coordinate, not guarantee, fiat payment finality.

---

## 2. Trust Assumptions

### 2.1 Fully Trusted (Local)

| Component | Trust Basis |
|-----------|-------------|
| User's own application code | Runs on user's machine; user controls execution |
| User's wallet private keys | Held locally; never transmitted |
| Local state storage | File-based; user controls access |

### 2.2 Partially Trusted (External Services)

| Component | Trust Basis | Risk if Compromised |
|-----------|-------------|---------------------|
| Bitcoin node (Electrum RPC) | User can self-host; third-party servers are unverified | Misleading transaction observations; delayed confirmations |
| Monero node (daemon RPC) | User can self-host; third-party servers are unverified | Misleading transaction observations; privacy leakage |
| Bank/PSP APIs | Regulated but not protocol-guaranteed | False payment detection; delayed transfers |

### 2.3 Untrusted (Participant-Provided)

| Component | Trust Basis | Risk if Malicious |
|-----------|-------------|-------------------|
| Counterparty messages | Adversarial; may be malformed or misleading | Protocol exploitation; fund loss |
| Counterparty bank details | Participant-provided; format-validated only | Payment to wrong address; phishing |
| Counterparty crypto address | Participant-provided; format-validated only | Payment to wrong address |
| Payment claims | Participant assertion; not independently verified | False payment claims |

### 2.4 Critical Trust Assumptions

| Assumption | If Violated |
|------------|-------------|
| secp256k1 discrete log problem is hard | Bitcoin signatures can be forged |
| Ed25519 discrete log problem is hard | Monero signatures can be forged |
| DLEQ proof construction is sound | Cross-curve key binding can be broken |
| SHA-256 is collision-resistant | Hash-based commitments can be broken |
| `thread_rng()` provides CSPRNG quality randomness | Key generation is predictable |
| Blockchain watchers report accurate state | Protocol makes decisions based on false data |
| Participants' clocks are approximately synchronized | Timestamp-based timeouts fail |

---

## 3. Attacker Capabilities

### 3.1 Malicious Counterparty

A malicious maker or taker can:

| Capability | Description | Mitigation |
|------------|-------------|------------|
| Send malformed messages | Craft invalid or adversarial protocol messages | Message validation; schema checks |
| Send duplicate messages | Replay previously sent messages | `MessageStore` replay protection |
| Send out-of-order messages | Violate expected message sequence | Sequence number validation |
| Disconnect strategically | Go offline at critical moments | `WaitingForPeer` state; recovery engine |
| Refuse to continue | Abandon swap after counterparty commits | Timeout mechanisms; refund paths |
| Submit false transaction info | Claim transactions exist when they don't | Blockchain verification via watchers |
| Exploit timeouts | Time actions to exploit timeout windows | Careful timeout design; symmetric timeouts |
| Replay messages across swaps | Use messages from one swap in another | Swap ID binding in messages |
| Impersonate another participant | Spoof sender identity | Participant ID validation |
| Exploit implementation vs spec gaps | Find differences between protocol spec and code | Thorough testing; formal verification |

### 3.2 Network Attacker

A network-level attacker (e.g., MITM on P2P connections) can:

| Capability | Description | Mitigation |
|------------|-------------|------------|
| Delay messages | Introduce latency in message delivery | Timeout mechanisms |
| Drop messages | Prevent messages from reaching recipient | Retry logic; liveness detection |
| Observe traffic | Monitor message patterns and timing | libp2p encryption |
| Inject messages | Attempt to inject forged messages | Message authentication; replay protection |

### 3.3 Blockchain Attacker

An attacker with significant blockchain resources can:

| Capability | Description | Mitigation |
|------------|-------------|------------|
| Delay confirmations | Withhold transaction propagation | Timeout-based recovery |
| Reorganize chains | Create conflicting chain histories | Reorg detection; confirmation depth |
| Dust attacks | Flood addresses with small transactions | Address filtering |
| Fee manipulation | Cause unexpected fee requirements | Fee estimation; fee bumping |

### 3.4 Compromised External Service

If an external service (node, bank API) is compromised:

| Capability | Description | Mitigation |
|------------|-------------|------------|
| False transaction reports | Claim transactions exist or don't | Cross-verify with multiple nodes |
| Delayed responses | Slow down swap progress | Timeout mechanisms |
| Privacy leakage | Reveal user's transaction patterns | Self-hosted nodes recommended |

---

## 4. Atomic Swap Threats

### 4.1 Pre-Swap Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Price manipulation** | Attacker posts offer at extreme price | Taker accepts unfavorable rate | Price validation; user confirmation |
| **Protocol version downgrade** | Attacker forces older protocol version | Weaker security properties | Version negotiation; minimum version enforcement |
| **DLEQ proof forgery** | Attacker provides invalid DLEQ proof | Counterparty cannot verify key binding | **Stub implementation — not currently mitigated** |

### 4.2 Funding Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Counterparty doesn't fund** | One party locks funds; other doesn't | Funds locked until timeout | Timeout-based refund |
| **Wrong amount funded** | Counterparty locks incorrect amount | Swap cannot complete | Amount verification before proceeding |
| **Front-running** | Attacker observes funding and acts first | Unfair advantage | Protocol is symmetric; no front-running window |
| **Transaction rejection** | Funding transaction rejected by network | Swap fails | Retry with adjusted fees |

### 4.3 Redemption Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Secret leakage** | Attacker obtains secret before redemption | Can claim funds prematurely | Secret only revealed during redemption |
| **Counterparty doesn't redeem** | One party claims; other doesn't | Partial completion | Timeout; recovery engine |
| **Race condition** | Both parties redeem simultaneously | Protocol state conflict | State machine enforcement |

### 4.4 Refund Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Refund blocked** | Counterparty prevents refund | Funds permanently locked | Refund is unilateral after timeout |
| **Refund race** | Both parties attempt refund | Double-spend attempt | Blockchain consensus prevents double-spend |
| **Timeout too short** | Legitimate party can't complete in time | Unnecessary refund | Careful timeout design |

### 4.5 Recovery Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Recovery decision wrong** | Engine chooses wrong action | Fund loss | Deterministic evaluation; blockchain verification |
| **Stale state recovery** | Recovery uses outdated blockchain state | Incorrect action | Fresh blockchain observations |
| **Recovery loop** | Engine repeatedly fails to recover | Resources wasted | Maximum retry limit; abort path |

---

## 5. Fiat Trade Threats

### 5.1 Payment Instruction Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Bank detail substitution** | Attacker changes bank details after trade starts | Payment goes to wrong address | Payment instructions bound to trade ID; change detection |
| **Phishing** | Attacker provides fake bank details | Taker pays attacker directly | User education; payment reference binding |
| **Impersonation** | Attacker pretends to be maker | Taker sends payment to attacker | Participant ID authentication |

### 5.2 Payment Claim Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **False payment claim** | Taker claims to have paid when they haven't | Maker releases crypto without payment | Maker confirmation required; payment adapter verification |
| **Payment screenshot forgery** | Taker provides fake proof of payment | Maker believes payment was sent | Adapter-based detection preferred over screenshots |
| **Delayed payment** | Taker sends payment after crypto is released | Maker receives crypto but not fiat | Payment deadline; dispute mechanism |
| **Partial payment** | Taker sends less than agreed amount | Maker receives insufficient funds | Amount verification |

### 5.3 Crypto Release Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Unauthorized release** | Third party attempts to release crypto | Funds stolen | 2-of-2 multisig requires both parties |
| **Release without payment** | Maker releases crypto before confirming payment | Maker loses funds | State machine enforces payment confirmation first |
| **Co-signing coercion** | One party refuses to co-sign return transaction | Funds stuck in escrow | Dispute mechanism; bond forfeiture |

### 5.4 Dispute Phase

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Frivolous disputes** | Party opens dispute to delay resolution | Counterparty's funds locked | Bond system penalizes bad faith |
| **Evidence tampering** | Party provides false evidence | Wrong dispute resolution | Evidence verification; cross-referencing |
| **Double dispute** | Party opens dispute twice | Protocol state confusion | State machine prevents duplicate disputes |

### 5.5 Post-Completion Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Payment reversal** | Fiat payment is reversed after crypto release | Maker loses both fiat and crypto | `ReversalDetected` state; dispute path |
| **Chargeback** | Taker disputes bank transfer after receiving crypto | Maker loses funds | Dispute mechanism; bond system |

---

## 6. Blockchain-Specific Threats

### 6.1 Bitcoin-Specific

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Delayed confirmations** | Transaction remains unconfirmed | Swap progress blocked | Timeout-based recovery |
| **Chain reorganization** | Confirmed transaction loses confirmations | Protocol state inconsistent | Reorg detection; minimum confirmation depth |
| **Fee market spike** | Unexpectedly high fees required | Transaction stuck | Fee estimation; fee bumping capability |
| **Node outage** | Electrum server unavailable | Cannot observe blockchain state | Multiple server support; self-hosted option |
| **Transaction rejection** | Transaction rejected by network | Swap fails | Error handling; retry with adjusted parameters |

### 6.2 Monero-Specific

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Privacy leakage** | View tags link transactions to swap | Privacy compromised | View tag design minimizes linkage |
| **Node sync delay** | Monero node not fully synchronized | Incorrect transaction observations | Sync status check; self-hosted node |
| **Ring signature malleability** | Transaction appears different to different observers | Confusion about transaction state | Canonical transaction representation |

### 6.3 Cross-Chain Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Asymmetric finality** | One chain confirms faster than other | Timing-based attacks | Independent timeout per chain |
| **Different fee models** | Fee estimation differs between chains | Unexpected costs | Per-chain fee estimation |
| **Different confirmation depths** | Bitcoin 6 vs Monero 10 confirmations | Different security assumptions | Chain-specific confirmation requirements |

---

## 7. Cryptographic Threats

### 7.1 DLEQ Proof Threats

**Critical:** The current DLEQ proof implementation is a **stub**. The following threats are not currently mitigated:

| Threat | Scenario | Impact | Status |
|--------|----------|--------|--------|
| **Proof forgery** | Attacker creates valid-looking but false DLEQ proof | Counterparty accepts invalid key binding | **Stub — not mitigated** |
| **Proof verification bypass** | Verification always returns true | No key binding guarantee | **Stub — not mitigated** |
| **Cross-curve key mismatch** | Different keys on secp256k1 and Ed25519 | Counterparty cannot verify same secret | **Stub — not mitigated** |

**Remediation:** Full Eigenwallet library integration required before production use.

### 7.2 Key Management Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Key leakage via logs** | Secret key written to log file | Private key exposed | Log redaction framework; no key logging |
| **Key leakage via crash dumps** | Core dump contains secret keys | Private key exposed | Keys zeroized on drop |
| **Weak randomness** | `thread_rng()` produces predictable values | Keys can be guessed | OS CSPRNG via `getrandom` crate |
| **Key persistence** | Keys accidentally written to disk | Private key exposed | In-memory only for secrets |

### 7.3 Multisig Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Key reuse** | Same escrow key used for multiple trades | Compromised key affects multiple trades | Per-trade key generation |
| **Partial signature leakage** | Partial signature exposed to third party | Could be used to reconstruct full signature | Secure transport; key zeroization |
| **Co-signing coercion** | One party refuses to co-sign | Funds stuck in escrow | Dispute mechanism; timeout |

---

## 8. Network and Transport Threats

### 8.1 P2P Network Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Sybil attack** | Attacker creates many fake peers | Network manipulation | Kademlia DHT limits; reputation system |
| **Eclipse attack** | Attacker isolates victim from honest peers | Complete control over victim's view | Multiple peer connections; peer diversity |
| **Message flooding** | Attacker floods network with messages | Denial of service | Rate limiting; message size limits |
| **Protocol downgrade** | Attacker forces older protocol version | Weaker security | Minimum version enforcement |

### 8.2 Message-Level Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Replay attack** | Attacker replays old message | Duplicate action | `MessageStore` replay protection |
| **Message tampering** | Attacker modifies message in transit | Invalid state transition | libp2p encryption; message authentication |
| **Message injection** | Attacker injects forged message | Unauthorized action | Participant ID validation; sender verification |
| **Timestamp manipulation** | Attacker sets future/past timestamps | Bypass freshness check | Server-side timestamp validation |

### 8.3 Transport Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Connection hijacking** | Attacker takes over P2P connection | Full control over messages | libp2p authentication |
| **Traffic analysis** | Attacker observes message patterns | Privacy leakage | libp2p encryption |
| **Denial of service** | Attacker floods connection | Service disruption | Rate limiting; connection limits |

---

## 9. Application-Level Threats

### 9.1 Authentication Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Password brute force** | Attacker guesses user password | Account takeover | Argon2id hashing; rate limiting |
| **Token theft** | Attacker steals HMAC-signed token | Account takeover | Token expiration; secure storage |
| **Session hijacking** | Attacker takes over active session | Account takeover | Session binding; token rotation |

### 9.2 Data Storage Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Plaintext credentials** | RPC passwords stored in plaintext | Credential exposure | Zeroization on drop; in-memory only |
| **State file exposure** | Swap state files contain sensitive data | Information leakage | File permissions; encryption at rest (future) |
| **Log data leakage** | Sensitive data written to logs | Information exposure | Log redaction framework |

### 9.3 GUI Threats

| Threat | Scenario | Impact | Mitigation |
|--------|----------|--------|------------|
| **Clipboard hijacking** | Attacker replaces copied address | Payment to wrong address | Address confirmation; checksum validation |
| **Phishing UI** | Fake GUI mimics real application | Credential theft | Application signing; user education |

---

## 10. Failure Scenarios

### 10.1 Participant Failures

| Scenario | Impact | Recovery |
|----------|--------|----------|
| Maker goes offline after funding | Swap stalls; taker's funds at risk | Timeout; recovery engine; refund |
| Taker goes offline after funding | Swap stalls; maker's funds at risk | Timeout; recovery engine; refund |
| Both parties go offline | Swap stalls indefinitely | Timeout; recovery on restart |
| Participant's wallet crashes | Cannot sign transactions | Wallet restart; state recovery |
| Participant's node disconnects | Cannot observe blockchain | Reconnect; catch-up on restart |

### 10.2 System Failures

| Scenario | Impact | Recovery |
|----------|--------|----------|
| Application crash during swap | State may be inconsistent | Event log replay; state reconstruction |
| Storage corruption | Swap state lost | Event log recovery; blockchain verification |
| Network partition | Parties cannot communicate | Timeout; recovery when partition heals |
| Blockchain node outage | Cannot observe transactions | Switch to alternative node; timeout |

### 10.3 Blockchain Failures

| Scenario | Impact | Recovery |
|----------|--------|----------|
| Transaction never confirms | Funds stuck in mempool | Replace-by-fee; abandon after timeout |
| Chain reorganization | Confirmed transaction loses confirmations | Reorg detection; wait for deeper confirmations |
| Blockchain halt | No new blocks produced | Timeout; manual intervention |

### 10.4 Fiat Payment Failures

| Scenario | Impact | Recovery |
|----------|--------|----------|
| Bank transfer delayed | Trade timeout; crypto released | Dispute mechanism; manual resolution |
| Bank transfer rejected | Taker's payment fails | Trade cancellation; refund |
| Payment reversal after completion | Maker loses funds | `ReversalDetected` state; dispute |
| Payment adapter false positive | Maker believes payment received | Maker confirmation required |
| Payment adapter false negative | Payment not detected | Manual payment confirmation |

---

## 11. Residual Risks

### 11.1 Unmitigated Risks

| Risk | Severity | Description | Status |
|------|----------|-------------|--------|
| **DLEQ proof stub** | Critical | DLEQ proof generation and verification are stubs; no actual key binding | **Not mitigated — requires Eigenwallet integration** |
| **Fiat payment non-atomicity** | High | Fiat and crypto settle independently; no atomic guarantee | **Inherent limitation — mitigated by state machine and dispute system** |
| **Bank detail substitution** | High | Attacker can change bank details after trade starts | **Partially mitigated — change detection implemented** |
| **False payment claims** | Medium | Taker can claim payment without sending it | **Mitigated by maker confirmation and payment adapters** |
| **Payment reversal** | Medium | Fiat payment can be reversed after crypto release | **Mitigated by `ReversalDetected` state and dispute path** |

### 11.2 Assumption-Dependent Risks

| Risk | Depends On | Impact if Violated |
|------|------------|-------------------|
| Blockchain finality | Sufficient confirmation depth | Double-spend; fund loss |
| Node accuracy | Honest blockchain nodes | Incorrect protocol decisions |
| CSPRNG quality | OS random number generator | Predictable keys; fund loss |
| Cryptographic hardness | secp256k1, Ed25519, SHA-256 security | Signature forgery; hash collision |
| Network delivery | P2P network reliability | Delayed or lost messages |

### 11.3 Operational Risks

| Risk | Description | Mitigation |
|------|-------------|------------|
| **User error** | User sends funds to wrong address | Address validation; confirmation dialogs |
| **Software bug** | Undiscovered code defect | Testing; formal verification; audit |
| **Dependency vulnerability** | Third-party library vulnerability | `cargo-deny`; supply chain monitoring |
| **Regulatory change** | Legal requirements change | Legal review; compliance monitoring |

---

## 12. Security Properties

### 12.1 Guaranteed Properties

| Property | Description | Evidence |
|----------|-------------|----------|
| **State machine enforcement** | Invalid transitions are rejected | `can_transition_to()` methods; unit tests |
| **Replay protection** | Duplicate messages are rejected | `MessageStore`; unique message IDs |
| **Key zeroization** | Secret keys are wiped from memory | `Drop` implementations using `zeroize` crate; 0 `unsafe` blocks |
| **Integer-only arithmetic** | No floating-point in monetary calculations | Type system; `Satoshis`, `Piconero`, `FiatAmount` |
| **Event sourcing** | Full state reconstruction from events | `EventRepository`; atomic writes |
| **2-of-2 multisig** | Both parties must co-sign escrow operations | `MultisigManager`; Bitcoin P2WSH |

### 12.2 Conditionally Guaranteed Properties

| Property | Condition | Description |
|----------|-----------|-------------|
| **Fund recovery** | Timeout elapses; blockchain accessible | Funds can be refunded after timeout |
| **Payment verification** | Payment adapter is accurate | Payments are detected correctly |
| **Message ordering** | Sequence numbers are respected | Messages processed in order |
| **Timestamp freshness** | Clocks are synchronized within 30 seconds | Stale messages are rejected |

### 12.3 Not Guaranteed

| Property | Reason |
|----------|--------|
| **Fiat payment finality** | Bank transfers are external to the protocol |
| **Counterparty honesty** | Protocol is designed for adversarial counterparties |
| **Blockchain availability** | External dependency; nodes can fail |
| **DLEQ proof soundness** | **Current implementation is a stub** |
| **Privacy** | P2P network traffic can be observed |
| **Regulatory compliance** | Depends on jurisdiction and use case |

---

## Appendix A: STRIDE Summary

| Category | Atomic Swap | Fiat Trade | Mitigation |
|----------|-------------|------------|------------|
| **Spoofing** | Counterparty impersonation | Bank detail substitution | Participant ID validation; authentication |
| **Tampering** | Message modification | Payment claim manipulation | State machine; message authentication |
| **Repudiation** | Deny swap participation | Deny payment sent/received | Event sourcing; payment adapters |
| **Information Disclosure** | Secret key leakage | Bank detail exposure | Key zeroization; log redaction |
| **Denial of Service** | Strategic disconnection | Dispute flooding | Timeouts; recovery; rate limiting |
| **Elevation of Privilege** | Unauthorized fund access | Unauthorized crypto release | 2-of-2 multisig; state machine |

---

## Appendix B: Risk Matrix

| Risk | Likelihood | Impact | Severity | Status |
|------|-----------|--------|----------|--------|
| DLEQ proof stub exploitation | High | Critical | **Critical** | Open |
| Fiat payment reversal | Medium | High | **High** | Mitigated |
| Bank detail substitution | Medium | High | **High** | Partially mitigated |
| False payment claim | Medium | High | **High** | Mitigated |
| Counterparty strategic disconnect | High | Medium | **Medium** | Mitigated |
| Blockchain node outage | Low | High | **Medium** | Mitigated |
| Replay attack | Low | Medium | **Medium** | Mitigated |
| Key leakage via logs | Low | High | **Medium** | Mitigated |
| Sybil attack | Medium | Low | **Low** | Mitigated |
| Timestamp manipulation | Low | Low | **Low** | Mitigated |
