# OpenDex Security Audit Package

This directory contains all materials needed for an independent external security audit of the OpenDex BTC–XMR atomic swap protocol.

## Project Overview

OpenDex is a peer-to-peer decentralized exchange enabling atomic swaps between Bitcoin (BTC) and Monero (XMR). The protocol uses cross-curve DLEQ proofs to enable trustless escrow without a trusted third party.

## Audit Scope

The recommended audit scope covers:

1. **Cryptography**: DLEQ proof generation and verification in `opendex-btc-xmr`
2. **Swap State Machine**: State transitions and invariants in `opendex-swap`
3. **Blockchain Integration**: Monero and Bitcoin chain interfaces in `opendex-monero` and `opendex-bitcoin`
4. **Recovery Logic**: Crash recovery and reconciliation in `opendex-recovery`
5. **P2P Transport**: Message routing and replay protection in `opendex-transport`
6. **Storage**: Event sourcing and persistence in `opendex-storage`

## Package Contents

| File | Description |
|------|-------------|
| `code_inventory.md` | Complete list of all 25 crates with descriptions |
| `threat_model.md` | Threat model covering swap protocol and P2P networking |
| `test_report.md` | Summary of all test coverage |
| `audit_scope.md` | Recommended audit scope and priorities |
| `audit_request_template.md` | Template for engaging external auditors |

## Contact

For questions about this audit package, please contact the OpenDex development team.
