//! OpenDex Desktop GUI
//!
//! Informational desktop application for the OpenDex P2P decentralized exchange.
//! Built with iced (Rust → native GUI).
//!
//! The GUI is a thin client of the user's local P2P node — it does not accept
//! orders, route trades, or provide a trading interface to a central exchange.
//!
//! ## Platform support
//!
//! Platform-specific code is organized under the `platforms` module:
//!
//! - `platforms::linux` — Linux-specific configuration and settings
//! - `platforms::windows` — Windows-specific configuration and settings
//! - `platforms::macos` — macOS-specific configuration and settings

pub mod model;
pub mod app;
pub mod style;
pub mod view;

/// Platform-specific modules — extend as needed for each OS.
pub mod platforms {
    pub mod linux;
    pub mod windows;
    pub mod macos;
}

// Re-exports for convenience
pub use model::{GuiModel, NodeUpdate, OrderBookView, TradeView, BalanceView, PeerView, TradeForm, NodeStatus};
pub use app::{Message, Flags, launch_gui, launch_gui_on_main, GuiCommand, GuiUpdate, BalanceUpdate};
