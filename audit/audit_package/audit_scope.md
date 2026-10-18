# Audit Scope & Priorities

## Priority 1: Critical — Must Pass Before Production

### DLEQ Proof Implementation (`opendex-btc-xmr`)
- Verify Fiat-Shamir challenge computation: `c = H(btc_pk || xmr_pk || R_btc || R_xmr)`
- Verify proof structure: `R_btc (32) || R_xmr (32) || s_btc (32) || s_xmr (32)`
- Verify `s*G - c*P == R` on ed25519 curve
- Verify cross-curve challenge binding prevents proof substitution
- Verify nonce generation uses OS RNG and is never reused
- Verify `derive_monero_public_key()` uses correct ed25519 clamping

### Swap State Machine (`opendex-swap`)
- Verify all state transitions are valid and enforced
- Verify no funds can be double-spent or double-redeemed
- Verify timeout handling correctly triggers refund
- Verify reconciliation correctly detects on-chain transactions

### Event Sourcing (`opendex-storage`, `opendex-events`)
- Verify append-only event log cannot be tampered with
- Verify event replay correctly reconstructs state
- Verify atomic writes prevent partial state corruption

## Priority 2: High — Should Pass Before Production

### Blockchain Integration (`opendex-bitcoin`, `opendex-monero`)
- Verify transaction lookup returns correct data
- Verify confirmation counting is accurate
- Verify broadcast handles rejection correctly
- Verify error handling for disconnected nodes

### Recovery Engine (`opendex-recovery`)
- Verify deterministic decisions match expected outcomes
- Verify funding confirmation thresholds (6 BTC, 10 XMR)
- Verify refund/redemption detection

### P2P Transport (`opendex-transport`)
- Verify replay protection via MessageStore
- Verify sequence number tracking
- Verify message ordering guarantees

## Priority 3: Medium — Nice to Have

### Monero Key Derivation
- Verify ed25519 scalar clamping matches Monero spec
- Verify basepoint multiplication is correct

### Fiat Settlement (`fiat`)
- Verify escrow logic
- Verify dispute resolution
- Verify payment verification

## Recommended Audit Duration

- **Critical**: 2-3 weeks for a dedicated crypto auditor
- **High**: 1-2 weeks for a systems auditor
- **Medium**: 1 week for a general Rust auditor

**Total estimated: 4-6 weeks**

## Known Issues to Review

See `audit/known_issues.md` for the current list of known issues, including:
- KNOWN-H02: Bitcoin transaction construction not yet tested end-to-end
- Any new findings from the DLEQ proof implementation
