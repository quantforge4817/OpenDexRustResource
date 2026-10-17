# Contributing to OpenDex

Thank you for your interest in contributing to OpenDex! We welcome contributions from the community. This guide will help you get started.

## Code of Conduct

By participating in this project, you are expected to uphold the standards of respectful and constructive communication. Be kind, be patient, and be collaborative.

## How to Contribute

### Reporting Bugs

If you find a bug, please [open an issue](https://github.com/quantforge4817/OpenDexRustResource/issues/new/choose) using the **Bug Report** template. Include:

- A clear description of the bug
- Steps to reproduce
- Expected vs. actual behavior
- Your environment (OS, Rust version, OpenDex version)
- Relevant logs or error output

### Suggesting Features

We welcome feature suggestions! Please [open an issue](https://github.com/quantforge4817/OpenDexRustResource/issues/new/choose) using the **Feature Request** template. Include:

- The problem the feature would solve
- Your proposed solution
- Use cases and benefits
- Alternative solutions you've considered

### Reporting Security Issues

**Do not** create public issues for security vulnerabilities. Use GitHub's [Security Advisories](https://github.com/quantforge4817/OpenDexRustResource/security/advisories/new) feature instead. See our [Security Policy](.github/SECURITY.md) for details.

## Development Setup

### Prerequisites

- **Rust 1.88+** (stable or nightly) — [Install Rust](https://rustup.rs/)
- **Cargo** package manager (included with Rust)
- **Trunk** (for web frontend development) — `cargo install trunk`
- **wasm32-unknown-unknown** target — `rustup target add wasm32-unknown-unknown`

### Building the Project

```bash
# Clone the repository
git clone https://github.com/quantforge4817/OpenDexRustResource.git
cd OpenDexRustResource

# Build the entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Run Clippy for linting
cargo clippy --workspace

# Check supply chain security
cargo deny check
```

### Running Components

```bash
# Run the P2P node
cargo run -p p2p-node

# Run the desktop GUI
cargo run -p desktop-gui

# Serve the web portal (with live reload)
cd crates/web-frontend
trunk serve --open
```

## Making Changes

1. **Fork** the repository
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```
3. **Make your changes** following the project's code style
4. **Write or update tests** for your changes
5. **Ensure all checks pass**:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo deny check
   ```
6. **Commit** your changes with a clear commit message
7. **Push** to your fork:
   ```bash
   git push origin feature/my-feature
   ```
8. **Open a Pull Request** using the PR template

## Code Style

- Follow the existing code style in the project
- Use `cargo fmt` to format your code
- Use `cargo clippy` to check for common mistakes
- Write clear, descriptive commit messages
- Add comments where the code's intent isn't obvious

## Pull Request Guidelines

- Keep PRs focused on a single feature or fix
- Reference related issues: `Closes #ISSUE_NUMBER`
- Include tests for new functionality
- Update documentation as needed
- Ensure CI checks pass before requesting review

## Project Structure

```
OpenDexRustResource/
├── crates/                 # Rust crates (23 total)
│   ├── common/             # Shared domain types
│   ├── protocol/           # Message types
│   ├── p2p-node/           # P2P node application
│   ├── opendex-*/          # Core protocol crates
│   ├── web-frontend/       # Leptos/WASM web portal
│   └── desktop-gui/        # iced desktop application
├── fuzz/                   # Fuzz testing targets
├── tests/                  # Integration tests
├── audit/                  # Security audit documentation
└── docs/                   # Documentation and deployment guides
```

## Architecture Overview

OpenDex consists of three main development milestones:

1. **BTC-XMR Atomic Swap Foundation** — Core types, protocol messages, Bitcoin/Monero chain operations, and swap orchestration
2. **Persistence, Watchers, Recovery** — Event sourcing, storage, chain watchers, and recovery mechanisms
3. **P2P Transport, Peer Discovery, Messaging** — Network transport layer and chain abstraction

Additional subsystems include DAO governance, fee/treasury management, user authentication, and fiat settlement.

## Testing

- Run all tests: `cargo test --workspace`
- Run tests for a specific crate: `cargo test -p crate-name`
- Run fuzz tests: `cargo fuzz run <target>`

## Documentation

- Update `README.md` for user-facing changes
- Update crate-level documentation with KDoc comments
- Update `docs/ROADMAP.md` for milestone changes
- Update `CHANGELOG.md` for notable changes

## Getting Help

- Join the **[Matrix Chat](https://matrix.to/#/#opendex:matrix.org)** for real-time discussion and support
- Check the [documentation](docs/) for guides and references
- Search existing [issues](https://github.com/quantforge4817/OpenDexRustResource/issues) for similar questions
- Open a new issue for questions or discussion

## License

By contributing, you agree that your contributions will be licensed under the project's dual **MIT** / **Apache-2.0** license.
