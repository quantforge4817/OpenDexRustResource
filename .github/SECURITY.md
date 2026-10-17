# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| v0.1.0  | :white_check_mark: |

## Reporting a Vulnerability

We take the security of OpenDex seriously. If you discover a security vulnerability, please report it responsibly.

### Reporting Process

1. **Do NOT** create a public GitHub issue for security vulnerabilities.
2. Use GitHub's [Security Advisories](https://github.com/quantforge4817/OpenDexRustResource/security/advisories/new) feature to report the vulnerability privately.
3. Alternatively, email **security@opendex.org** for direct contact.
4. For urgent matters, you can also reach us via **[Matrix Chat](https://matrix.to/#/#opendex:opendex.org)** (use encrypted messages).

### What to Include

When reporting a vulnerability, please include:

- A clear description of the vulnerability
- Steps to reproduce the issue
- The affected component/crate
- Potential impact assessment
- Any suggested mitigation or fix (if known)

### Response Timeline

- **Acknowledgment:** We will acknowledge receipt of your report within **48 hours**.
- **Assessment:** We will assess the vulnerability and provide an initial response within **5 business days**.
- **Resolution:** We will work to resolve the issue and keep you informed of progress.
- **Disclosure:** We will coordinate with you on public disclosure timing, following responsible disclosure practices.

### Security Scope

The following areas are in scope for security reports:

- Cryptographic implementations and key management
- Atomic swap protocol and smart contract logic
- P2P network protocol and message handling
- Authentication and authorization mechanisms
- Input validation and parsing
- Memory safety issues (buffer overflows, use-after-free, etc.)
- Supply chain vulnerabilities

### Security Best Practices

OpenDex follows these security practices:

- **Dependency auditing:** Regular `cargo-deny` checks for known vulnerabilities
- **Fuzz testing:** 7 fuzz targets for protocol message parsing and state machine validation
- **Static analysis:** `cargo clippy` and Miri for undefined behavior detection
- **Code review:** All changes require review before merging
- **Minimal unsafe code:** Only 2 `unsafe` blocks in the entire codebase, both for password zeroization

### Acknowledgments

We appreciate the security research community's help in making OpenDex more secure. Contributors who report valid security issues will be acknowledged in our security advisories (unless they prefer to remain anonymous).
