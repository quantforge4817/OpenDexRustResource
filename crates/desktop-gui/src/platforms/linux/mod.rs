//! Linux-specific desktop GUI configuration.
//!
//! This module provides Linux-specific settings for the OpenDex desktop application.
//!
//! ## Platform notes
//!
//! - Uses X11/Wayland display servers via iced's winit backend
//! - Supports system tray integration via StatusNotifierItem (SNI)
//! - File paths follow XDG Base Directory Specification

/// Default data directory for Linux (XDG_DATA_HOME/opendex)
pub fn default_data_dir() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|p| p.join("opendex"))
}

/// Default config directory for Linux (XDG_CONFIG_HOME/opendex)
pub fn default_config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("opendex"))
}
