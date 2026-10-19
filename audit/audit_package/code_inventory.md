# Code Inventory

## Core Swap Protocol (Priority: Critical)

| Crate | Description | Dependencies |
|-------|-------------|--------------|
| `opendex-btc-xmr` | Cross-curve DLEQ proofs, BTC↔XMR key derivation | k256, curve25519-dalek, secp256k1, sha2 |
| `opendex-swap` | Swap state machine, engine, reconciliation | opendex-btc-xmr, opendex-storage, opendex-recovery |
| `opendex-protocol-types` | Shared types: Swap, SwapState, ProtocolMessage | opendex-types, opendex-errors |
| `opendex-types` | Core types: SwapId, Satoshis, Piconero, ParticipantId | uuid, serde |
| `opendex-errors` | Error types: ChainError, ProtocolError, StorageError | thiserror |

## Blockchain Integration (Priority: High)

| Crate | Description | Dependencies |
|-------|-------------|--------------|
| `opendex-bitcoin` | Bitcoin chain via Electrum RPC | electrum-client, bitcoin |
| `opendex-monero` | Monero chain via daemon RPC | monero-rpc, tokio |
| `opendex-watchers` | Chain watchers for BTC and XMR | opendex-bitcoin, opendex-monero |
| `opendex-chain` | Chain abstraction layer | opendex-bitcoin, opendex-monero |

## Persistence & Recovery (Priority: High)

| Crate | Description | Dependencies |
|-------|-------------|--------------|
| `opendex-storage` | File-based swap/event/tx repositories | serde_json, tokio |
| `opendex-events` | Event sourcing types and records | opendex-protocol-types |
| `opendex-recovery` | Deterministic recovery engine | opendex-protocol-types |

## P2P Transport (Priority: Medium)

| Crate | Description | Dependencies |
|-------|-------------|--------------|
| `opendex-transport` | Transport trait, MockTransport, libp2p transport | libp2p, async-trait |
| `opendex-protocol` | Protocol registry and version negotiation | opendex-protocol-types |

## Supporting Crates (Priority: Low)

| Crate | Description | Audit Status |
|-------|-------------|--------------|
| `opendex-dao` | DAO governance interface | ✓ Audited (61 tests) |
| `opendex-fee-treasury` | Fee and treasury subsystem | ✓ Audited (25 tests) |
| `common` | Shared utilities | ✓ Audited |
| `protocol` | Legacy protocol types | ✓ Audited |
| `user` | User accounts, auth, reputation | ✓ Audited (39 tests) |
| `fiat` | Fiat settlement subsystem | ✓ Audited (88 tests) |
| `p2p-node` | Full P2P node implementation | ✓ Audited (23 tests) |
| `web-frontend` | Leptos web portal | 🟢 Audited 2026-10-18: Low risk, 3 findings (WF-001 through WF-003) |
| `desktop-gui` | Iced desktop application | 🟡 Audited 2026-10-18: Medium risk, 8 findings (DG-001 through DG-008) |

## Test Infrastructure

| Path | Description |
|------|-------------|
| `crates/opendex-swap/tests/swap_e2e.rs` | 10 E2E integration tests |
| `crates/opendex-transport/tests/multi_node.rs` | 4 multi-node adversarial tests |
| `fuzz/fuzz_targets/` | 12 fuzz targets |
| `crates/p2p-node/tests/` | 16 security and integration tests |
