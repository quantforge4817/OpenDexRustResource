# OpenDex Desktop Application

Native desktop application for the OpenDex P2P decentralized exchange, built with Rust and iced.

## Overview

The OpenDex Desktop Application is a standalone desktop client that embeds both the P2P node and the GUI in a single process. It runs on Linux, Windows, and macOS.

The GUI is a **thin client of the user's local P2P node** — it does not accept orders, route trades, or provide a trading interface to a central exchange. This design maintains a **GREEN** regulatory rating.

## Features

- **Order Book** — View current bids and asks
- **Trade Form** — Place buy/sell orders with price and amount
- **Trade History** — Monitor active trades and their state
- **Wallet Panel** — View balances and fund test accounts
- **Peer Management** — Connect to P2P peers
- **Node Status Bar** — Monitor node identity and connections
- **On-chain Wallets** — Optional Monero and Bitcoin wallet integration

## Prerequisites

- Rust 1.88+ (stable or nightly)
- Platform-specific GUI dependencies:
  - **Linux**: X11 or Wayland display server with development headers
  - **Windows**: MSVC build tools (included with Rust toolchain)
  - **macOS**: Xcode command line tools

### Linux Dependencies

**Debian/Ubuntu:**
```bash
sudo apt install libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev pkg-config
```

**Fedora/RHEL:**
```bash
sudo dnf install libX11-devel libXrandr-devel libXi-devel mesa-libGL-devel pkg-config
```

## Running

### Development

```bash
# From the project root
cargo run -p desktop-gui
```

### With Options

```bash
cargo run -p desktop-gui -- --port 4001
cargo run -p desktop-gui -- --peer /ip4/127.0.0.1/tcp/4001/p2p/PEER_ID
cargo run -p desktop-gui -- --new-identity
cargo run -p desktop-gui -- --monero-rpc http://127.0.0.1:18082
cargo run -p desktop-gui -- --btc-rpc http://127.0.0.1:8332
```

### Command-Line Options

| Option | Description |
|--------|-------------|
| `--port <PORT>` | Listening TCP port [default: 4001] |
| `--peer <MULTIADDR>` | Dial a peer on startup |
| `--new-identity` | Generate a new node identity |
| `--monero-rpc <URL>` | Monero wallet RPC URL |
| `--monero-rpc-user <U>` | Monero wallet RPC username |
| `--monero-rpc-pass <P>` | Monero wallet RPC password |
| `--btc-rpc <URL>` | Bitcoin wallet RPC URL |
| `--btc-rpc-user <U>` | Bitcoin wallet RPC username |
| `--btc-rpc-pass <P>` | Bitcoin wallet RPC password |
| `-h, --help` | Print help message |

## Building

### Linux

```bash
# Debug build
./build/build-linux.sh

# Release build (creates tar.gz archive)
./build/build-linux.sh --release
```

### Windows

```cmd
REM Debug build
build\build-windows.bat

REM Release build (creates zip archive)
build\build-windows.bat release
```

### Cross-Compilation (Windows from Linux)

```bash
# Install the Windows target
rustup target add x86_64-pc-windows-msvc

# Install the mingw linker
sudo apt install mingw-w64

# Build for Windows
cargo build --release -p desktop-gui --target x86_64-pc-windows-msvc
```

The Windows binary will be at `target/x86_64-pc-windows-msvc/release/opendex-desktop.exe`.

## Architecture

```
opendex-desktop (binary)
│
├── tokio runtime (main thread)
│   ├── P2P Node (p2p-node crate)
│   │   ├── libp2p swarm
│   │   ├── Order book
│   │   ├── Trade book
│   │   ├── Balance book
│   │   └── Wallet clients (optional)
│   │
│   └── Bridge adapters
│       ├── GUI commands → Node bridge
│       └── Node updates → GUI
│
└── iced GUI thread
    ├── Status bar
    ├── Order book panel
    ├── Trade form
    ├── Trades panel
    ├── Peers panel
    └── Wallet panel
```

## Project Structure

```
crates/desktop-gui/
├── Cargo.toml
├── README.md
├── build/
│   ├── build-linux.sh
│   └── build-windows.bat
└── src/
    ├── lib.rs          # Library exports
    ├── main.rs         # Standalone binary entry point
    ├── app.rs          # GUI app logic (Message, update, view, launch_gui)
    ├── model.rs        # GUI view model (GuiModel, OrderBookView, etc.)
    ├── style.rs        # Dark theme colors
    ├── view/           # UI widgets
    │   ├── mod.rs
    │   ├── status_bar.rs
    │   ├── order_book.rs
    │   ├── trade_form.rs
    │   ├── trades.rs
    │   ├── wallet.rs
    │   └── peers.rs
    └── platforms/      # Platform-specific configuration
        ├── linux/
        ├── windows/
        └── macos/
```

## Regulatory Compliance

The desktop GUI is rated **🟢 GREEN** under the OpenDex regulatory compliance framework:

- The GUI is a local node client, not a gateway to a central exchange
- No orders are routed through a central service
- No user data is collected or transmitted to third parties
- All P2P communication is encrypted via libp2p

See `audit/regulatory_compliance/regulatory_compliance.md` for the full compliance analysis.

## License

See the project root for the applicable OpenDex license and project licensing information.
