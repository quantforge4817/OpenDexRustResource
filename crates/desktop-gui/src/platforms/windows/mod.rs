//! Windows-specific desktop GUI configuration.
//!
//! This module provides Windows-specific settings for the OpenDex desktop application.
//!
//! ## Platform notes
//!
//! - Uses Windows Desktop Window Manager (DWM) via iced's winit backend
//! - Supports Windows taskbar integration
//! - File paths follow Windows AppData conventions

/// Default data directory for Windows (%LOCALAPPDATA%/OpenDex)
pub fn default_data_dir() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|p| p.join("OpenDex"))
}

/// Default config directory for Windows (%APPDATA%/OpenDex)
pub fn default_config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("OpenDex"))
}
