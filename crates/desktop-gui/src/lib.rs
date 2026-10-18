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

pub mod app;
pub mod model;
pub mod style;
pub mod view;

/// Platform-specific modules — extend as needed for each OS.
pub mod platforms {
    pub mod linux;
    pub mod macos;
    pub mod windows;
}

// Re-exports for convenience
pub use app::{
    BalanceUpdate, Flags, GuiCommand, GuiUpdate, Message, launch_gui, launch_gui_on_main,
};
pub use model::{
    BalanceView, GuiModel, NodeStatus, NodeUpdate, OrderBookView, PeerView, TradeForm, TradeView,
};
