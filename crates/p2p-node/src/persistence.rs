use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(path)?)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write(path, bytes)
}

pub fn read<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn write<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let data = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &data)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("persistence");

    let temp_path = parent.join(format!(".{}.{}.tmp", file_name, Uuid::new_v4()));

    let result = (|| -> Result<()> {
        let mut file = File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        fs::rename(&temp_path, path)?;

        #[cfg(unix)]
        {
            File::open(parent)?.sync_all()?;
        }

        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}
