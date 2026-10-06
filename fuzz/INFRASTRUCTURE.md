# OpenDEX Fuzzing Infrastructure Setup

This document describes how to set up and use the fuzzing infrastructure for the OpenDEX Rust codebase.

For the detailed fuzzing process, targets, and commands, see [FUZZ.md](./FUZZ.md).

---

## 1. Prerequisites

- **Rust toolchain**: Check with `rustc --version` and `cargo --version`.
- **LLVM**: Required by libFuzzer. On most Linux distros: `sudo apt install llvm`. On macOS, it comes with Xcode command-line tools.
- **cargo-fuzz CLI**: Install once with `cargo install cargo-fuzz`.

---

## 2. Initialize the Fuzz Workspace

From the project root, run:

```bash
cargo fuzz init
```

This creates:

```
fuzz/
├── Cargo.toml          (fuzzing workspace config)
└── fuzz_targets/
    └── fuzz_target_1.rs  (delete this file)
```

The `fuzz/Cargo.toml` will look like:

```toml
[package]
name = "opendexrustsource-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"

[dependencies.protocol]
path = "../crates/protocol"

[[bin]]
name = "fuzz_target_1"
path = "fuzz_targets/fuzz_target_1.rs"
test = false
doc = false
```

---

## 3. Required Dependencies

The fuzz workspace `Cargo.toml` needs:

| Dependency | Purpose |
|---|---|
| `libfuzzer-sys = "0.4"` | The fuzzing engine (LLVM libFuzzer bindings) |
| `protocol` (local path `../crates/protocol`) | Your protocol crate with message types |
| `serde_json` | For JSON deserialization in fuzz targets |

You may also want to add `serde` and `uuid` if your targets construct objects directly.

---

## 4. Directory Structure

After setup, your structure will be:

```
OpenDexRustSource/
├── Cargo.toml
├── crates/
│   ├── common/
│   ├── protocol/
│   └── p2p-node/
├── fuzz/                          # Fuzzing workspace
│   ├── Cargo.toml
│   ├── FUZZ.md                    # Fuzzing process guide
│   ├── INFRASTRUCTURE.md          # This file
│   ├── fuzz_targets/
│   │   ├── message.rs
│   │   ├── order.rs
│   │   ├── trade.rs
│   │   ├── trade_state.rs
│   │   └── state_response.rs
│   ├── corpus/                    # Auto-created at runtime
│   │   ├── message/
│   │   ├── order/
│   │   ├── trade/
│   │   ├── trade_state/
│   │   └── state_response/
│   └── artifacts/                 # Auto-created on crash
│       ├── message/
│       └── ...
```

---

## 5. Fuzz Target Files

Each target is a standalone binary. Five targets are planned:

| Target | File | Description |
|---|---|---|
| `message` | `fuzz_targets/message.rs` | Outer network message parser |
| `order` | `fuzz_targets/order.rs` | Nested Order JSON payloads |
| `trade` | `fuzz_targets/trade.rs` | Trade JSON payloads |
| `trade_state` | `fuzz_targets/trade_state.rs` | Trade state machine transitions |
| `state_response` | `fuzz_targets/state_response.rs` | Nested Vec<Order> and Vec<Trade> |

See [FUZZ.md](./FUZZ.md) sections 7, 14, 15, 16, and 20.1 for the full code for each target.

---

## 6. What Happens at Build Time

When you run `cargo fuzz run <target>`:

1. `cargo-fuzz` compiles the fuzz target in **release mode** with special flags.
2. LLVM's `libFuzzer` is linked statically.
3. The resulting binary runs in a tight loop, mutating input and measuring code coverage.
4. No network, no database, no external services — pure in-process fuzzing.

---

## 7. Optional Enhancements

| Tool | Purpose |
|---|---|
| **AddressSanitizer (ASan)** | Detect memory errors (use `-Zsanitizer=address`) |
| **cargo-audit** | Scan dependencies for known CVEs (`cargo install cargo-audit`) |
| **cargo-llvm-cov** | Measure code coverage of fuzz targets |

---

## 8. Quick Start Commands

```bash
# One-time setup
cargo install cargo-fuzz
cargo fuzz init

# Create the fuzz targets
# (edit fuzz/fuzz_targets/ to add targets from FUZZ.md)

# Run a 5-minute smoke test
cargo fuzz run message -- -max_total_time=300
```

---

## 9. What You Don't Need

- No Docker containers or VMs for basic fuzzing.
- No test database or network infrastructure.
- No changes to the main `Cargo.toml` workspace — the fuzz workspace is isolated.
- No modifications to the `protocol` or `p2p-node` crates.

The fuzzing infrastructure is entirely self-contained under `fuzz/` and reads from your existing crates without modifying them.
