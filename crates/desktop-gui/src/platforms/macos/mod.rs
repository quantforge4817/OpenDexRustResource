//! macOS-specific desktop GUI configuration.
//!
//! This module provides macOS-specific settings for the OpenDex desktop application.
//!
//! ## Platform notes
//!
//! - Uses AppKit via iced's winit backend
//! - Supports macOS menu bar integration
//! - File paths follow macOS Application Support conventions

/// Default data directory for macOS (~/Library/Application Support/OpenDex)
pub fn default_data_dir() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|p| p.join("OpenDex"))
}

/// Default config directory for macOS (~/Library/Application Support/OpenDex)
pub fn default_config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("OpenDex"))
}
