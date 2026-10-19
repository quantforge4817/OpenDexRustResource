//! Encryption at rest for stored state files.
//!
//! Uses AES-256-GCM for authenticated encryption with HKDF-SHA256 key derivation.
//! All persisted data is encrypted on disk, protecting against unauthorized file access.
//!
//! # Key Derivation
//!
//! A master password (from environment variable) is combined with a per-file salt
//! using HKDF-SHA256 to derive a unique 256-bit encryption key per file.
//! The salt is stored alongside the encrypted data (first 16 bytes of the file).
//!
//! # File Format
//!
//! ```text
//! [8 bytes magic][16 bytes salt][12 bytes nonce][encrypted data + 16 bytes auth tag]
//! ```
//!
//! The magic bytes are `OPNDEX01` for definitive detection of encrypted files.
//! The nonce is randomly generated per write operation. The auth tag is appended
//! by AES-GCM automatically.

use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use opendex_errors::StorageError;
use sha2::Sha256;

/// Size of the per-file salt in bytes.
const SALT_LEN: usize = 16;

/// Size of the AES-GCM nonce in bytes.
const NONCE_LEN: usize = 12;

/// Size of the AES-GCM authentication tag in bytes.
const TAG_LEN: usize = 16;

/// Magic bytes at the start of every encrypted file for definitive detection.
const MAGIC_BYTES: &[u8] = b"OPNDEX01";
const MAGIC_LEN: usize = 8;

/// Derive a 256-bit encryption key from a master password and per-file salt.
fn derive_key(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"opendex-storage"), password);
    let mut okm = [0u8; 32];
    hk.expand(salt, &mut okm)
        .expect("HKDF expand with fixed output length should not fail");
    okm
}

/// Generate a random salt for a new file.
fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    getrandom::getrandom(&mut salt).expect("Failed to generate random salt");
    salt
}

/// Generate a random nonce for AES-GCM.
fn generate_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).expect("Failed to generate random nonce");
    nonce
}

/// Encrypt plaintext data using AES-256-GCM.
///
/// Returns the encrypted blob: `[salt][nonce][ciphertext][tag]`.
pub fn encrypt(password: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, StorageError> {
    let salt = generate_salt();
    let nonce = generate_nonce();
    let key = derive_key(password, &salt);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| StorageError::Io(format!("Failed to create AES-GCM cipher: {}", e)))?;

    let ciphertext = cipher
        .encrypt(&Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| StorageError::Io(format!("AES-GCM encryption failed: {}", e)))?;

    // Build the encrypted blob: magic || salt || nonce || ciphertext (tag is appended by cipher)
    let mut blob = Vec::with_capacity(MAGIC_LEN + SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(MAGIC_BYTES);
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// Decrypt an encrypted blob produced by `encrypt`.
///
/// Expects the format: `[magic][salt][nonce][ciphertext][tag]`.
pub fn decrypt(password: &[u8], blob: &[u8]) -> Result<Vec<u8>, StorageError> {
    // Check for magic bytes
    if blob.len() < MAGIC_LEN || &blob[0..MAGIC_LEN] != MAGIC_BYTES {
        return Err(StorageError::Io(
            "Encrypted blob does not start with magic bytes".to_string(),
        ));
    }

    let data = &blob[MAGIC_LEN..];
    if data.len() < SALT_LEN + NONCE_LEN + TAG_LEN {
        return Err(StorageError::Io(
            "Encrypted blob too short to contain valid data".to_string(),
        ));
    }

    let salt = &data[0..SALT_LEN];
    let nonce = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext_with_tag = &data[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, salt);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| StorageError::Io(format!("Failed to create AES-GCM cipher: {}", e)))?;

    let plaintext = cipher
        .decrypt(&Nonce::from_slice(nonce), ciphertext_with_tag)
        .map_err(|e| StorageError::Io(format!("AES-GCM decryption failed: {}", e)))?;

    Ok(plaintext)
}

/// Check if a file is encrypted by examining its magic bytes header.
///
/// Encrypted files start with the `OPNDEX01` magic bytes. Files that don't
/// start with these bytes are considered unencrypted (e.g., empty files or
/// legacy plain-text files).
pub fn is_encrypted_file(path: &Path) -> bool {
    let blob = match std::fs::read(path) {
        Ok(blob) => blob,
        Err(_) => return false,
    };
    blob.starts_with(MAGIC_BYTES)
}

/// Read and decrypt a file, returning the plaintext bytes.
///
/// If the file appears unencrypted (too short), reads it as plain text.
pub fn read_encrypted_file(
    password: &[u8],
    path: &Path,
) -> Result<Vec<u8>, StorageError> {
    let blob = std::fs::read(path).map_err(|e| {
        StorageError::Io(format!("Failed to read file {}: {}", path.display(), e))
    })?;

    if blob.is_empty() {
        return Ok(Vec::new());
    }

    if is_encrypted_file(path) {
        decrypt(password, &blob)
    } else {
        // Legacy unencrypted file — return as-is
        Ok(blob)
    }
}

/// Encrypt and atomically write data to a file.
pub fn write_encrypted_file(
    password: &[u8],
    path: &Path,
    plaintext: &[u8],
) -> Result<(), StorageError> {
    let blob = encrypt(password, plaintext)?;
    crate::storage::atomic_write(path, &blob)?;
    Ok(())
}

/// Encrypt and atomically append a line to an encrypted log file.
///
/// For append-only logs, this reads all existing records, decrypts them,
/// appends the new line, re-encrypts, and writes atomically.
/// This is less efficient than true append but maintains encryption integrity.
pub fn append_encrypted_line(
    password: &[u8],
    path: &Path,
    line: &[u8],
) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| {
        StorageError::Io(format!("Failed to create directory: {}", e))
    })?;

    // Read existing content
    let mut plaintext = Vec::new();
    if path.exists() {
        let blob = std::fs::read(path).map_err(|e| {
            StorageError::Io(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        if !blob.is_empty() && is_encrypted_file(path) {
            plaintext = decrypt(password, &blob)?;
        } else if !blob.is_empty() {
            plaintext = blob;
        }

        // Ensure newline separator
        if !plaintext.is_empty() && !plaintext.ends_with(b"\n") {
            plaintext.push(b'\n');
        }
    }

    plaintext.extend_from_slice(line);
    write_encrypted_file(password, path, &plaintext)?;
    Ok(())
}

/// Master key provider trait for obtaining the encryption password.
///
/// Implementations can read from environment variables, key stores,
/// or hardware security modules.
pub trait MasterKeyProvider: Send + Sync {
    /// Return the master encryption password bytes.
    fn password(&self) -> Result<Vec<u8>, StorageError>;
}

/// Environment variable-based master key provider.
///
/// Reads the encryption password from the specified environment variable.
#[derive(Debug, Clone)]
pub struct EnvMasterKeyProvider {
    /// Name of the environment variable containing the master password.
    var_name: String,
}

impl EnvMasterKeyProvider {
    /// Create a new provider that reads from the given environment variable.
    pub fn new(var_name: impl Into<String>) -> Self {
        Self {
            var_name: var_name.into(),
        }
    }
}

impl MasterKeyProvider for EnvMasterKeyProvider {
    fn password(&self) -> Result<Vec<u8>, StorageError> {
        std::env::var(&self.var_name).map(|s| s.into_bytes()).map_err(|_| {
            StorageError::Io(format!(
                "Encryption password environment variable '{}' not set",
                self.var_name
            ))
        })
    }
}

/// Encrypted storage wrapper that provides encrypted read/write operations.
///
/// Wraps the file path and master key provider to provide transparent
/// encryption for all storage operations.
#[derive(Debug)]
pub struct EncryptedStorage<P: MasterKeyProvider> {
    provider: P,
}

impl<P: MasterKeyProvider> EncryptedStorage<P> {
    /// Create a new encrypted storage wrapper.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Encrypt and write data to a file atomically.
    pub fn write(&self, path: &Path, data: &[u8]) -> Result<(), StorageError> {
        let password = self.provider.password()?;
        write_encrypted_file(&password, path, data)
    }

    /// Read and decrypt data from a file.
    pub fn read(&self, path: &Path) -> Result<Vec<u8>, StorageError> {
        let password = self.provider.password()?;
        read_encrypted_file(&password, path)
    }

    /// Append an encrypted line to a log file.
    pub fn append_line(&self, path: &Path, line: &[u8]) -> Result<(), StorageError> {
        let password = self.provider.password()?;
        append_encrypted_line(&password, path, line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only MasterKeyProvider that holds a password directly, avoiding unsafe env var access.
    #[derive(Debug, Clone)]
    struct TestMasterKeyProvider {
        password: Vec<u8>,
    }

    impl TestMasterKeyProvider {
        fn new(password: &[u8]) -> Self {
            Self {
                password: password.to_vec(),
            }
        }
    }

    impl MasterKeyProvider for TestMasterKeyProvider {
        fn password(&self) -> Result<Vec<u8>, StorageError> {
            Ok(self.password.clone())
        }
    }

    fn test_password() -> Vec<u8> {
        b"test-master-password".to_vec()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = test_password();
        let plaintext = b"Hello, encrypted world!";

        let encrypted = encrypt(&password, plaintext).unwrap();
        // Encrypted data should be longer than plaintext (salt + nonce + tag)
        assert!(encrypted.len() > plaintext.len());

        let decrypted = decrypt(&password, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_password_fails() {
        let password = test_password();
        let wrong_password = b"wrong-password".to_vec();
        let plaintext = b"Secret data";

        let encrypted = encrypt(&password, plaintext).unwrap();
        let result = decrypt(&wrong_password, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted_data_fails() {
        let password = test_password();
        let plaintext = b"Secret data";

        let mut encrypted = encrypt(&password, plaintext).unwrap();
        // Corrupt a byte in the ciphertext (after magic + salt + nonce)
        encrypted[MAGIC_LEN + SALT_LEN + NONCE_LEN] ^= 0xFF;

        let result = decrypt(&password, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let password = test_password();
        let plaintext = b"";

        let encrypted = encrypt(&password, plaintext).unwrap();
        let decrypted = decrypt(&password, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_plaintext() {
        let password = test_password();
        let plaintext = vec![0xAB; 1024 * 1024]; // 1 MB

        let encrypted = encrypt(&password, &plaintext).unwrap();
        let decrypted = decrypt(&password, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_blob_too_short() {
        let password = test_password();
        let short_blob = vec![0u8; 10];
        let result = decrypt(&password, &short_blob);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_deterministic() {
        let password = b"test";
        let salt = [0u8; 16];
        let key1 = derive_key(password, &salt);
        let key2 = derive_key(password, &salt);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_salt() {
        let password = b"test";
        let salt1 = [0u8; 16];
        let salt2 = [1u8; 16];
        let key1 = derive_key(password, &salt1);
        let key2 = derive_key(password, &salt2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_file_roundtrip() {
        let password = test_password();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        let plaintext = b"{\"key\": \"value\"}";

        write_encrypted_file(&password, &path, plaintext).unwrap();
        assert!(is_encrypted_file(&path));

        let decrypted = read_encrypted_file(&password, &path).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_append_encrypted_line() {
        let password = test_password();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.log");

        append_encrypted_line(&password, &path, b"line1").unwrap();
        append_encrypted_line(&password, &path, b"line2").unwrap();

        let decrypted = read_encrypted_file(&password, &path).unwrap();
        assert_eq!(decrypted, b"line1\nline2");
    }

    #[test]
    fn test_master_key_provider() {
        let provider = TestMasterKeyProvider::new(b"test-pass");
        let password = provider.password().unwrap();
        assert_eq!(password, b"test-pass");
    }

    #[test]
    fn test_env_master_key_provider_missing() {
        let provider = EnvMasterKeyProvider::new("NONEXISTENT_VAR");
        let result = provider.password();
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_storage_wrapper() {
        let provider = TestMasterKeyProvider::new(b"wrapper-test");

        let storage = EncryptedStorage::new(provider);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wrapped.json");

        storage.write(&path, b"wrapped data").unwrap();
        let data = storage.read(&path).unwrap();
        assert_eq!(data, b"wrapped data");
    }

    #[test]
    fn test_magic_bytes_detection() {
        let password = test_password();
        let dir = tempfile::tempdir().unwrap();

        // Encrypted file should be detected
        let encrypted_path = dir.path().join("encrypted.json");
        write_encrypted_file(&password, &encrypted_path, b"secret").unwrap();
        assert!(is_encrypted_file(&encrypted_path));

        // Plain text file should not be detected as encrypted
        let plaintext_path = dir.path().join("plaintext.json");
        std::fs::write(&plaintext_path, b"{\"key\": \"value\"}").unwrap();
        assert!(!is_encrypted_file(&plaintext_path));

        // Empty file should not be detected as encrypted
        let empty_path = dir.path().join("empty.json");
        std::fs::write(&empty_path, b"").unwrap();
        assert!(!is_encrypted_file(&empty_path));

        // Non-existent file should not be detected as encrypted
        let missing_path = dir.path().join("missing.json");
        assert!(!is_encrypted_file(&missing_path));
    }

    #[test]
    fn test_decrypt_rejects_missing_magic_bytes() {
        let password = test_password();
        // A blob without magic bytes should be rejected
        let no_magic_blob = vec![0u8; 100];
        let result = decrypt(&password, &no_magic_blob);
        assert!(result.is_err());
    }
}
