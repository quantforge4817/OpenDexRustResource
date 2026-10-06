//! Common storage utilities and error types.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use opendex_errors::StorageError;

/// Atomic write: write to a temp file, sync, then rename.
/// On failure the temp file is cleaned up.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("persistence");

    let temp_path = parent.join(format!(".{}.{}.tmp", file_name, Uuid::new_v4()));

    let result = (|| -> Result<(), StorageError> {
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

/// Atomic append: read existing lines, append new line, rewrite atomically.
pub fn atomic_append_line(path: &Path, line: &[u8]) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("persistence");

    let temp_path = parent.join(format!(".{}.{}.tmp", file_name, Uuid::new_v4()));

    let result = (|| -> Result<(), StorageError> {
        // Read existing content
        let mut content = Vec::new();
        if path.exists() {
            content = fs::read(path)?;
            // Ensure newline separator if content is non-empty and doesn't end with newline
            if !content.is_empty() && !content.ends_with(b"\n") {
                content.push(b'\n');
            }
        }
        content.extend_from_slice(line);

        let mut file = File::create(&temp_path)?;
        file.write_all(&content)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

/// Derive a file path from a base directory and an ID string.
pub fn id_to_path(base: &Path, id: &str, extension: &str) -> PathBuf {
    base.join(format!("{}.{}", id, extension))
}
