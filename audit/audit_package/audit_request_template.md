# Audit Request Template

## Project: OpenDex BTC–XMR Atomic Swap Protocol

### Overview

We are seeking an independent security audit of the OpenDex atomic swap protocol, which enables trustless BTC↔XMR swaps using cross-curve DLEQ proofs.

### Scope

The audit should cover:

1. **Cryptography**: DLEQ proof generation and verification (secp256k1 ↔ ed25519)
2. **Swap Protocol**: State machine, escrow logic, and fund flow
3. **Blockchain Integration**: Monero daemon RPC and Bitcoin Electrum RPC
4. **Recovery**: Crash recovery and reconciliation logic
5. **P2P Transport**: Message routing and replay protection

### Deliverables Expected

- Detailed audit report with findings categorized by severity
- Code review comments and suggested fixes
- Assessment of cryptographic correctness
- Assessment of financial logic (escrow, balances, state transitions)
- Recommendations for hardening

### Timeline

- **Kickoff**: [Date]
- **Audit Period**: 4-6 weeks
- **Report Delivery**: [Date]
- **Remediation Period**: 2-4 weeks
- **Final Sign-off**: [Date]

### Access

- Full repository access via GitHub
- Audit package in `audit/audit_package/`
- Direct access to development team for questions

### Budget

[To be discussed]

### Recommended Auditors

- Specialized crypto audit firms (e.g., Trail of Bits, NCC Group, OpenZeppelin)
- Rust-focused security firms
- Monero/Bitcoin protocol specialists

### Contact

[Contact information]
