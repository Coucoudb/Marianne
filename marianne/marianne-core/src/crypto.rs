//! Field-level AES-256-GCM encryption for sensitive SQLite fields.
//!
//! Key derivation: SHA-256(passphrase || fixed_salt).
//! Ciphertext format (base64url-no-pad encoded): `nonce(12 bytes) || ciphertext+tag`

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

const KEY_SALT: &[u8] = b"marianne-history-v1";

/// Derive a 32-byte AES-256 key from a passphrase using SHA-256(passphrase || salt).
///
/// # Security note
/// This is a single-pass SHA-256 hash, **not** a password-hardened KDF.
/// It is adequate when `passphrase` is a high-entropy secret (e.g. a random
/// 32-byte hex string stored in `MARIANNE_DB_KEY`).  For low-entropy
/// passphrases a proper KDF such as Argon2id or PBKDF2-HMAC-SHA256 should be
/// used instead — switching requires a data-migration step and is tracked as a
/// future improvement.
pub fn derive_key(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    hasher.update(KEY_SALT);
    hasher.finalize().into()
}

/// Encrypt `plaintext` with AES-256-GCM using a random nonce.
/// Returns a base64url-no-pad string: `nonce(12) || ciphertext+tag`.
pub fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<String> {
    let aes_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Aes256Gcm::generate_nonce(&mut rand::rngs::OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("AES-GCM encryption failed"))?;

    let mut payload = nonce.to_vec();
    payload.extend_from_slice(&ciphertext);
    Ok(URL_SAFE_NO_PAD.encode(payload))
}

/// Decrypt a base64url-no-pad string produced by [`encrypt`].
pub fn decrypt(key: &[u8; 32], ciphertext_b64: &str) -> Result<String> {
    let payload = URL_SAFE_NO_PAD
        .decode(ciphertext_b64)
        .context("base64 decode failed")?;

    if payload.len() < 28 {
        anyhow::bail!("ciphertext too short (minimum 28 bytes: 12 nonce + 16 tag)");
    }

    let (nonce_bytes, ciphertext) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let aes_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(aes_key);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("AES-GCM decryption failed"))?;

    String::from_utf8(plaintext_bytes).context("decrypted bytes are not valid UTF-8")
}

/// Returns the database encryption key.
/// Reads `MARIANNE_DB_KEY` env var; falls back to a key derived from the machine hostname.
/// Never logs the key value.
///
/// # Security note — hostname fallback
/// When `MARIANNE_DB_KEY` is absent the key is derived from `gethostname()`.
/// Hostnames are often predictable (e.g. `localhost`, `DESKTOP-XXXXXX`), so
/// an attacker who obtains the SQLite file can attempt offline brute-force
/// with common hostnames.  **Set `MARIANNE_DB_KEY` in production** to a
/// randomly-generated, high-entropy secret.
pub fn get_db_key() -> [u8; 32] {
    if let Ok(passphrase) = std::env::var("MARIANNE_DB_KEY") {
        if !passphrase.is_empty() {
            return derive_key(&passphrase);
        }
    }
    let hostname = gethostname::gethostname()
        .to_string_lossy()
        .into_owned();
    derive_key(&hostname)
}

/// Returns `true` if `s` appears to be an encrypted value:
/// it base64url-decodes to at least 28 bytes (12-byte nonce + 16-byte tag minimum).
/// Used as a backward-compatibility guard so existing plaintext rows are returned as-is.
pub fn is_encrypted(s: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(s)
        .map(|bytes| bytes.len() >= 28)
        .unwrap_or(false)
}
