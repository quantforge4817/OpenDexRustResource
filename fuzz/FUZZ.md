# OpenDEX Rust Fuzzing Process

## 1\. Purpose

This document defines the standard fuzz-testing process for the OpenDEX Rust codebase.

The goals are to:

- Discover panics and crashes caused by malformed input.
- Test protocol deserialization against arbitrary attacker-controlled data.
- Exercise order, trade, and state-machine boundaries.
- Identify arithmetic and boundary-condition defects.
- Convert discovered fuzzing failures into permanent regression tests.
- Maintain reproducible fuzzing commands and documented results.

Fuzzing is a security-testing activity and should be performed against a development/test instance of OpenDEX.

---

# 2\. Repository

The OpenDEX Rust workspace currently contains:

```
OpenDexRustSource/
├── Cargo.toml
├── crates/
│   ├── common/
│   ├── p2p-node/
│   │   ├── src/
│   │   └── tests/
│   └── protocol/
│       └── src/
├── audit/
├── docs/
├── fuzz/
│   ├── FUZZ.md
│   └── INFRASTRUCTURE.md
└── tests/
```

The primary security-relevant areas for fuzzing are:

```
crates/protocol/
crates/p2p-node/
```

The network message flow currently uses JSON serialization.

The relevant network boundary is:

```
Peer/network bytes
        │
        ▼
serde_json::from_slice::<Message>()
        │
        ▼
Message
        │
        ▼
msg.payload
        │
        ▼
serde_json::from_str::<Order/Trade/...>()
        │
        ▼
OpenDEX state processing
```

---

# 3\. Fuzzing Tool

OpenDEX uses Rust's `cargo-fuzz` workflow with LLVM libFuzzer.

Install the tool:

```
cargo install cargo-fuzz
```

Verify:

```
cargo fuzz --help
```

The project should be built with a Rust toolchain compatible with the workspace.

Check:

```
rustc --version
cargo --version
```

---

# 4\. Initial Repository Checks

Always begin from the repository root:

```
cd /data/OpenDexFolder/OpenDexRustSource
```

Check the working tree:

```
git status
```

Run the normal test suite before fuzzing:

```
cargo test --workspace
```

Build the workspace:

```
cargo build --workspace
```

The fuzzing baseline should be established only after the normal test suite is passing or existing failures have been documented.

---

# 5\. Inspect the Protocol

Before creating a fuzz target, identify how network data is serialized and deserialized.

For OpenDEX:

```
grep -R "serde_json\|bincode\|postcard\|from_slice\|from_str\|to_vec\|to_string" \
    -n crates --exclude-dir=target
```

The current implementation uses `serde_json`.

Important parsing locations include:

```
crates/p2p-node/src/message_handler.rs   (receive-side deserialization)
crates/p2p-node/src/network.rs           (send-side serialization only)
```

Note: `network.rs` serializes outgoing messages via `to_vec()` and does not deserialize incoming data. The primary receive-side deserialization boundary is `message_handler.rs`.

The primary network parser is:

```
serde_json::from_slice::<Message>(data)
```

Nested payloads are subsequently parsed into protocol objects such as:

```
Order
Trade
TradeStateChanged
OrderCancelled
StateSnapshot
```

---

# 6\. Initialize cargo-fuzz

From the repository root:

```
cargo fuzz init
```

This creates the fuzzing workspace:

```
fuzz/
├── Cargo.toml
└── fuzz_targets/
    └── fuzz_target_1.rs
```

Do not commit generated crash artifacts blindly. Review them first.

---

# 7\. First Fuzz Target — Message

The first target should test the outer network message parser.

Create:

```
fuzz/fuzz_targets/message.rs
```

Contents:

```
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::Message;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Message>(data);
});
```

The target intentionally accepts arbitrary bytes.

The expected behaviour is:

```
valid input   → successfully parsed
invalid input → normal deserialization error
malformed data → no panic
```

A fuzz input must not cause an unexpected process crash.

---

# 8\. Run the First Fuzz Test

Run:

```
cargo fuzz run message
```

For a short controlled test:

```
cargo fuzz run message -- -max_total_time=60
```

For a longer test:

```
cargo fuzz run message -- -max_total_time=3600
```

This runs for approximately one hour.

For a specific number of fuzzing iterations:

```
cargo fuzz run message -- -runs=100000
```

---

# 9\. Recommended Initial Fuzzing Schedule

For development:

```
cargo fuzz run message -- -max_total_time=300
```

This gives a five-minute smoke test.

For a security test:

```
cargo fuzz run message -- -max_total_time=3600
```

For extended testing:

```
cargo fuzz run message -- -max_total_time=21600
```

Six hours is useful for scheduled or overnight testing.

For continuous fuzzing, run the target for as long as practical.

---

# 10\. Fuzzing Output

A successful fuzzing run may report statistics such as:

```
stat::number_of_executed_units
stat::average_exec_per_sec
stat::new_units_added
```

The important result is that the target completes without a crash.

A crash normally results in a file under:

```
fuzz/artifacts/
```

For example:

```
fuzz/artifacts/message/crash-<hash>
```

Never delete a crash artifact before investigating it.

---

# 11\. Reproducing a Crash

If libFuzzer finds a crash, first reproduce it using the generated artifact.

Example:

```
cargo fuzz run message \
    fuzz/artifacts/message/crash-<hash>
```

Replace `<hash>` with the actual artifact name.

The crash should be reproduced before making code changes.

Record:

```
Date:
Target:
Command:
Crash artifact:
Commit:
Rust version:
Operating system:
Observed panic:
Affected source file:
```

---

# 12\. Preserve the Failing Input

Copy the failing input to a clearly identified location if required by the project.

For example:

```
fuzz/corpus/message/
```

Do not modify the original crash artifact until the failure has been understood.

A useful workflow is:

```
mkdir -p fuzz/corpus/message
cp fuzz/artifacts/message/<crash-file> \
   fuzz/corpus/message/
```

Then rerun the target against the corpus:

```
cargo fuzz run message
```

---

# 13\. Convert Bugs Into Regression Tests

Every confirmed fuzzing bug should eventually have a normal deterministic regression test.

The regression test should:

1. Reproduce the original failure.
2. Assert the corrected behaviour.
3. Remain in the normal test suite.
4. Be documented in the audit/security log.

Example pattern:

```
#[test]
fn regression_<description>() {
    let input = /* original failing input */;

    let result = /* affected parser/function */;

    assert!(result.is_err());
}
```

The exact assertion depends on the defect.

The goal is that fixing the fuzzing failure does not permanently depend on the fuzzing infrastructure.

---

# 14\. Second Target — Nested Order Payloads

After the outer message parser, fuzz the protocol objects carried inside `Message`.

Example target:

```
fuzz/fuzz_targets/order.rs
```

Code:

```
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::Order;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Order>(data);
});
```

Run:

```
cargo fuzz run order -- -max_total_time=3600
```

This specifically exercises malformed order JSON.

---

# 15\. Third Target — Trade Payloads

Create:

```
fuzz/fuzz_targets/trade.rs
```

Code:

```
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::Trade;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<Trade>(data);
});
```

Run:

```
cargo fuzz run trade -- -max_total_time=3600
```

Trade objects deserve particular attention because they contain:

```
id (Uuid)
buy_order (Uuid)
sell_order (Uuid)
maker (String)
taker (String)
buyer (String)
seller (String)
market (Market)
price (u64)
amount (u64)
fill_sequence (u64)
state (TradeState)
created_at (u64)
updated_at (u64)
```

---

# 16\. Fourth Target — Trade State Machine

The `TradeState` transition system should also be fuzzed.

The security property is:

```
invalid transition
        ↓
returns an error
        ↓
trade state is not corrupted
```

while:

```
valid transition
        ↓
returns Ok
        ↓
state changes correctly
```

The target should generate sequences of `TradeState` values and timestamps and repeatedly call:

```
trade.transition(next, timestamp)
```

Create:

```
fuzz/fuzz_targets/trade_state.rs
```

Code:

```
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::{Trade, TradeState};

fuzz_target!(|data: &[u8]| {
    // Parse a sequence of TradeState indices from arbitrary bytes.
    // Each byte maps to one of the 13 TradeState variants.
    let states: Vec<TradeState> = data.iter()
        .filter_map(|&b| {
            match b % 13 {
                0 => Some(TradeState::Created),
                1 => Some(TradeState::Accepted),
                2 => Some(TradeState::EscrowFunding),
                3 => Some(TradeState::EscrowFunded),
                4 => Some(TradeState::PaymentPending),
                5 => Some(TradeState::PaymentSent),
                6 => Some(TradeState::PaymentConfirmed),
                7 => Some(TradeState::PayoutPending),
                8 => Some(TradeState::Settled),
                9 => Some(TradeState::Cancelled),
                10 => Some(TradeState::Expired),
                11 => Some(TradeState::Disputed),
                _ => None,
            }
        })
        .collect();

    if states.is_empty() {
        return;
    }

    // Build a minimal trade starting in Created state.
    let mut trade = Trade {
        id: Default::default(),
        buy_order: Default::default(),
        sell_order: Default::default(),
        maker: "seller".to_string(),
        taker: "buyer".to_string(),
        buyer: "buyer".to_string(),
        seller: "seller".to_string(),
        market: Default::default(),
        price: 0,
        amount: 0,
        fill_sequence: 0,
        state: TradeState::Created,
        created_at: 1,
        updated_at: 1,
    };

    let mut timestamp: u64 = 2;
    for next in &states {
        // Invalid transitions must return an error, not panic.
        let _ = trade.transition(next.clone(), timestamp);
        timestamp += 1;
    }
});
```

Important cases include:

```
Created
Accepted
EscrowFunding
EscrowFunded
PaymentPending
PaymentSent
PaymentConfirmed
PayoutPending
Settled
Cancelled
Expired
Disputed
```

The fuzz target uses a custom input struct — a sequence of state indices derived from raw bytes — rather than simple JSON deserialization, because `Trade::transition()` takes a `TradeState` enum variant directly, not a JSON string. This approach exercises all 13 states plus duplicate transitions, since the implementation explicitly treats duplicate state events as idempotent.

---

# 17\. Financial Boundary Values

OpenDEX uses:

```
pub const PRICE_SCALE: u64 = 100_000_000;
pub const AMOUNT_SCALE: u64 = 100_000_000;
```

Fuzzing should deliberately exercise numeric boundaries.

Important values include:

```
0
1
2
99_999_999
100_000_000
100_000_001
u32::MAX
u64::MAX
```

The fuzzing process should look for:

```
integer overflow
integer underflow
division by zero
incorrect scaling
unexpected truncation
invalid zero values
unexpected maximum values
```

Where arithmetic is performed, fuzz the operation itself rather than only fuzzing JSON parsing.

---

# 18\. Network Message Handler

After parser-level fuzzing is stable, fuzz the higher-level message handling path.

Relevant source:

```
crates/p2p-node/src/message_handler.rs
```

The important boundary is approximately:

```
serde_json::from_slice::<Message>(data)
```

followed by payload-specific parsing.

Higher-level fuzzing should verify that malformed peer messages:

```
do not panic
do not corrupt state
do not bypass authentication
do not create invalid orders
do not create invalid trades
do not perform unauthorized state transitions
```

This target should use a controlled test context rather than starting a real production node.

---

# 19\. Authentication Boundary

OpenDEX already contains security tests involving the relationship between:

```
message.sender
```

and:

```
authenticated_peer_id
```

This is an important fuzzing boundary.

Fuzzing should investigate variations such as:

```
missing sender
empty sender
incorrect sender
very long sender
malformed sender
valid sender with malformed payload
valid payload from wrong sender
```

The expected security property is that message contents cannot simply claim to originate from another authenticated peer.

---

# 20\. State Synchronization

The code contains:

```
StateRequest
StateResponse
```

and network handling for state synchronization.

These should eventually receive dedicated fuzz targets.

Important cases include:

```
empty order list
empty trade list
very large order list
very large trade list
duplicate objects
conflicting objects
invalid UUIDs
invalid timestamps
invalid states
extreme prices
extreme amounts
```

The objective is to ensure that untrusted state synchronization data cannot crash or corrupt the local node.

---

# 20.1 StateResponse Fuzz Target

`StateResponse` is a high-value fuzz target because it contains nested collections of `Order` and `Trade` objects, exercising both parsers in a single deserialization call.

Create:

```
fuzz/fuzz_targets/state_response.rs
```

Code:

```
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::message::StateResponse;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<StateResponse>(data);
});
```

Run:

```
cargo fuzz run state_response -- -max_total_time=3600
```

This target exercises:

```
empty orders and trades vectors
single-element vectors
very large vectors (memory pressure)
malformed nested Order objects
malformed nested Trade objects
conflicting UUIDs between orders and trades
invalid market definitions
extreme price and amount values
```

---

# 20.2 P2P Offer Fuzz Target

`P2POffer` is a network-facing type broadcast to peers when a user posts a P2P fiat-to-crypto offer. It contains crypto asset type, side, amount, price, fiat currency, payment method, and payment instructions.

Create:

```
fuzz/fuzz_targets/p2p_offer.rs
```

Code:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::p2p::P2POffer;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<P2POffer>(data);
});
```

Run:

```
cargo fuzz run p2p_offer -- -max_total_time=3600
```

This target exercises:

```
malformed P2P offer JSON
invalid CryptoAsset variants (XMR, BTC)
invalid Side variants (Buy, Sell)
extreme price and amount values
invalid FiatCurrency variants (USD, EUR, GBP, Custom)
invalid PaymentMethod variants (BankTransfer, Cash, Revolut, Custom)
overflow UUID fields
truncated or corrupted payment instructions strings
```

---

# 20.3 P2P Trade Fuzz Target

`P2PTrade` is a network-facing type broadcast when a P2P offer is taken. It contains all offer fields plus trade state, escrow address, transaction hashes, and payment proofs.

Create:

```
fuzz/fuzz_targets/p2p_trade.rs
```

Code:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::p2p::P2PTrade;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<P2PTrade>(data);
});
```

Run:

```
cargo fuzz run p2p_trade -- -max_total_time=3600
```

This target exercises:

```
malformed P2P trade JSON
invalid P2PTradeState variants
corrupted escrow addresses and transaction hashes
extreme fiat and crypto amounts
invalid Optional fields (escrow_tx_hash, payment_id, fiat_payment_proof, release_tx_hash)
conflicting offer_id and trade_id UUIDs
```

---

# 20.4 P2P Trade State Fuzz Target

`P2PTradeState` is a state machine with 9 states governing P2P fiat-to-crypto trade lifecycle. Invalid transitions must return errors, not panic.

Create:

```
fuzz/fuzz_targets/p2p_trade_state.rs
```

Code:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol::p2p::{P2PTrade, P2PTradeState};

fuzz_target!(|data: &[u8]| {
    let states: Vec<P2PTradeState> = data
        .iter()
        .filter_map(|&b| match b % 9 {
            0 => Some(P2PTradeState::OfferTaken),
            1 => Some(P2PTradeState::EscrowFunded),
            2 => Some(P2PTradeState::FiatPaymentPending),
            3 => Some(P2PTradeState::FiatPaymentSent),
            4 => Some(P2PTradeState::FiatPaymentConfirmed),
            5 => Some(P2PTradeState::CryptoReleased),
            6 => Some(P2PTradeState::Settled),
            7 => Some(P2PTradeState::Cancelled),
            _ => Some(P2PTradeState::Disputed),
        })
        .collect();

    if states.is_empty() {
        return;
    }

    let mut trade = P2PTrade { /* minimal instance */ };
    let mut timestamp: u64 = 2;
    for next in &states {
        let _ = trade.transition(next.clone(), timestamp);
        timestamp += 1;
    }
});
```

Run:

```
cargo fuzz run p2p_trade_state -- -max_total_time=3600
```

This target exercises:

```
all valid state transitions (OfferTaken → EscrowFunded → … → Settled)
all invalid state transitions (e.g., OfferTaken → Settled)
repeated identical state transitions (idempotency)
rapid state oscillation between valid neighbors
exceptional paths (Cancelled, Disputed, Expired)
```

---

# 21\. Corpus Management

A corpus contains interesting inputs discovered during fuzzing.

Typical structure:

```
fuzz/
├── corpus/
│   ├── message/
│   ├── order/
│   ├── trade/
│   ├── trade_state/
│   ├── state_response/
│   ├── p2p_offer/
│   ├── p2p_trade/
│   └── p2p_trade_state/
├── artifacts/
└── fuzz_targets/
```

Run a target against its corpus normally:

```
cargo fuzz run message
```

Add known security-relevant examples to the corpus when appropriate.

The corpus should contain representative protocol inputs rather than enormous numbers of redundant files.

---

# 22\. Minimize a Failing Input

If a fuzz input is large, libFuzzer can minimize it.

A minimized artifact is easier to understand, reproduce, and turn into a regression test.

Use:

```
cargo fuzz tmin message fuzz/artifacts/message/<crash-file>
```

Keep the original crash artifact until the minimized input has been verified.

---

# 23\. Run All Fuzz Targets

Once multiple targets exist, run each independently.

Example:

```
cargo fuzz run message -- -max_total_time=3600
cargo fuzz run order -- -max_total_time=3600
cargo fuzz run trade -- -max_total_time=3600
cargo fuzz run trade_state -- -max_total_time=3600
cargo fuzz run state_response -- -max_total_time=3600
cargo fuzz run p2p_offer -- -max_total_time=3600
cargo fuzz run p2p_trade -- -max_total_time=3600
cargo fuzz run p2p_trade_state -- -max_total_time=3600
```

Do not interpret one successful target as proof that the entire system has been fuzz-tested.

Each target covers a different attack surface.

---

# 24\. Normal Test Suite After Fuzzing

After modifying code in response to a fuzz finding:

```
cargo fmt --all -- --check
```

Then:

```
cargo check --workspace
```

Then:

```
cargo test --workspace
```

Then rerun the affected fuzz target:

```
cargo fuzz run <target> -- -max_total_time=300
```

For security fixes, use a longer run after the short verification succeeds.

---

# 25\. Security Finding Procedure

For every confirmed fuzzing issue:

### Step 1 — Preserve

Keep:

```
crash artifact
fuzz input
command
commit hash
```

### Step 2 — Reproduce

Confirm the crash is deterministic.

### Step 3 — Identify

Determine:

```
source file
function
input condition
root cause
security impact
```

### Step 4 — Fix

Implement the smallest appropriate correction.

### Step 5 — Regression Test

Add a deterministic test.

### Step 6 — Refuzz

Run the affected target again.

### Step 7 — Document

Record the result in:

```
audit/BUG_LOG.md
audit/AUDIT_CHECKLIST.md
audit/SECURITY_LOG.md
```

Use whichever file corresponds to the project's existing audit process.

---

# 26\. Fuzzing Log Template

For each fuzzing session record:

```
OpenDEX Fuzzing Session
========================

Date:
Operator:
Git commit:
Rust version:
Cargo version:
OS:

Target:
Command:

Duration:
Executions:
New corpus inputs:

Crashes:
Timeouts:

Result:
PASS / FAIL

Findings:

Regression test added:
Yes / No

Follow-up:
```

---

# 27\. Example Five-Minute Security Smoke Test

From the repository root:

```
cargo test --workspace
```

Then:

```
cargo fuzz run message -- -max_total_time=300
```

Then:

```
cargo fuzz run order -- -max_total_time=300
```

Then:

```
cargo fuzz run trade -- -max_total_time=300
```

Then:

```
cargo fuzz run trade_state -- -max_total_time=300
```

Then:

```
cargo fuzz run state_response -- -max_total_time=300
```

Finally:

```
cargo test --workspace
```

Record the results.

---

# 28\. Example One-Hour Protocol Test

```
cargo fuzz run message -- -max_total_time=3600
```

Follow with:

```
cargo fuzz run order -- -max_total_time=3600
```

Then:

```
cargo fuzz run trade -- -max_total_time=3600
```

Then:

```
cargo fuzz run trade_state -- -max_total_time=3600
```

Then:

```
cargo fuzz run state_response -- -max_total_time=3600
```

---

# 29\. Extended / Overnight Test

For an extended run:

```
cargo fuzz run message -- -max_total_time=21600
```

Six hours:

```
cargo fuzz run order -- -max_total_time=21600
```

Six hours:

```
cargo fuzz run trade -- -max_total_time=21600
```

Six hours:

```
cargo fuzz run trade_state -- -max_total_time=21600
```

Six hours:

```
cargo fuzz run state_response -- -max_total_time=21600
```

For unattended testing, redirect output to a log:

```
cargo fuzz run message -- -max_total_time=21600 \
    2>&1 | tee fuzz-message.log
```

---

# 30\. Completion Criteria

A fuzzing target is considered successfully tested for a given session when:

- The target builds successfully.
- The normal test suite is passing.
- The fuzz target runs for the documented duration.
- No unexpected crash occurs.
- No sanitizer failure occurs.
- Any discovered crash has been reproduced and investigated.
- Confirmed defects have regression tests.
- Security-relevant findings have been documented.
- The fuzzing command and duration have been recorded.

A successful fuzzing session does **not** prove the absence of vulnerabilities.

It only establishes that the tested input space and execution duration did not expose a failure under the tested conditions.

---

# 31\. Recommended OpenDEX Fuzzing Priority

The initial priority should be:

```
1. Message JSON parser
2. Order JSON parser
3. Trade JSON parser
4. TradeState transition logic
5. StateResponse parser (nested Vec<Order> and Vec<Trade>)
6. TradeStateChanged parser
7. OrderCancelled parser
8. Network message handler
9. Authentication/message-sender validation
10. State synchronization
11. Order-book invariants
12. Financial arithmetic and boundary conditions
```

This ordering should be revisited as the protocol and network architecture evolve.

---

# 32\. Standard Command Reference

### Build

```
cargo build --workspace
```

### Check

```
cargo check --workspace
```

### Tests

```
cargo test --workspace
```

### Format

```
cargo fmt --all -- --check
```

### Install cargo-fuzz

```
cargo install cargo-fuzz
```

### Initialize

```
cargo fuzz init
```

### List targets

```
cargo fuzz list
```

### Run target

```
cargo fuzz run <target>
```

### Five-minute run

```
cargo fuzz run <target> -- -max_total_time=300
```

### One-hour run

```
cargo fuzz run <target> -- -max_total_time=3600
```

### Six-hour run

```
cargo fuzz run <target> -- -max_total_time=21600
```

### Fixed iteration count

```
cargo fuzz run <target> -- -runs=100000
```

### Reproduce crash

```
cargo fuzz run <target> fuzz/artifacts/<target>/<artifact>
```

### Minimize crash

```
cargo fuzz tmin <target> fuzz/artifacts/<target>/<artifact>
```

---

# 33\. Security Principle

OpenDEX receives data from other peers.

Therefore:

> Treat every network message as untrusted input.

Fuzzing should progressively move from:

```
raw bytes
    ↓
JSON parser
    ↓
protocol object
    ↓
message validation
    ↓
authentication
    ↓
business logic
    ↓
state transition
    ↓
persistence
```

The objective is not merely to prove that JSON parsing does not panic. The long-term objective is to demonstrate that malformed or adversarial peer input cannot cause unsafe state transitions, unauthorized actions, crashes, or corruption of exchange state.

The **first practical phase** for the current codebase is therefore:

```
cd /data/OpenDexFolder/OpenDexRustSource

cargo test --workspace

cargo install cargo-fuzz

cargo fuzz init
```

Then we create the first `message` target and run a 5-minute baseline. After that, build the **Order → Trade → state-machine → message-handler** targets in that order.
