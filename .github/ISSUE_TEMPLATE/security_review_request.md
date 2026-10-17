---
name: Request for independent BTC/XMR protocol security review
about: Invite independent developers to review the atomic swap protocol implementation
title: "Request for independent BTC/XMR protocol security review"
labels: security, review
---

## Overview

We are seeking independent review, contributions, and critical feedback from Bitcoin, Monero, Rust, and cryptographic protocol developers.

The goal is to establish what works, identify weaknesses, and improve the design before any consideration of production deployment.

**Please report suspected vulnerabilities privately using the repository's [security reporting instructions](.github/SECURITY.md) rather than publishing exploitable details immediately.**

## Review Focus Areas

We ask reviewers to focus first on:

1. **Atomic-swap correctness** — Verify that the BTC ↔ XMR swap protocol guarantees atomicity under all conditions.
2. **Refund safety** — Ensure that funds are always recoverable if a swap fails or a counterparty disappears.
3. **Adaptor-signature assumptions** — Validate the cryptographic assumptions and their implementation.
4. **Failure recovery** — Test that the system handles network partitions, node crashes, and unexpected state transitions gracefully.

## Testing on Test Networks

Developers are invited to test on **Bitcoin testnet** and **Monero stagenet only**.

- Before testing, please verify that the test setup **cannot accidentally use mainnet funds**.
- See [`docs/testnet/`](docs/testnet/) for the testnet deployment guide and configuration.

**Do not use mainnet funds.** This project is not yet ready for production deployment.

## Security Status

> **Important:** Do not advertise or treat this exchange as safe or ready for trading until the relevant security work has been completed. Independent audit and thorough testing are required before any production use.

## How to Contribute

1. Review the code in the relevant crates (`opendex-swap/`, `opendex-protocol/`, `opendex-bitcoin/`, `opendex-monero/`)
2. Run the test suite: `cargo test --workspace`
3. Set up a local testnet: see [`docs/testnet/`](docs/testnet/)
4. Report findings via private security reports or discuss in issues/PRs
5. Join the discussion on **[Matrix Chat](https://matrix.to/#/#opendex:matrix.org)**
