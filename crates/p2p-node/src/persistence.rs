use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::Path;

pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(path)?)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::write(path, bytes)?;
    Ok(())
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
    let data = serde_json::to_string_pretty(value)?;
    fs::write(path, data)?;

    Ok(())
}
