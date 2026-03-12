use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::str::FromStr;

/// Derives a 32-byte encryption key from a password using SHA-256.
pub fn derive_encryption_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encrypts a private key string using AES-256-GCM with a password-derived key.
/// Returns a base64-encoded string containing the 12-byte nonce prepended to the
/// ciphertext.
pub fn encrypt_key(private_key: &str, password: &str) -> Result<String> {
    let key_bytes = derive_encryption_key(password);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, private_key.as_bytes())
        .map_err(|e| anyhow::anyhow!("encryption failed: {}", e))?;

    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypts a base64-encoded encrypted private key using a password-derived key.
pub fn decrypt_key(encrypted: &str, password: &str) -> Result<String> {
    let combined = BASE64
        .decode(encrypted)
        .context("failed to decode base64")?;

    if combined.len() < 13 {
        bail!("encrypted data too short");
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key_bytes = derive_encryption_key(password);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed (wrong password?): {}", e))?;

    String::from_utf8(plaintext).context("decrypted key is not valid UTF-8")
}

/// Derives an Ethereum address from a hex private key string.
/// Returns a checksummed 0x-prefixed address.
pub fn address_from_key(private_key: &str) -> Result<String> {
    let key = private_key.strip_prefix("0x").unwrap_or(private_key);
    let signer = alloy::signers::local::PrivateKeySigner::from_str(key)
        .context("invalid private key")?;
    Ok(format!("{:?}", signer.address()))
}

/// Formats an address for display: "0x1234...abcd" (first 6 + last 4 chars).
pub fn format_address(address: &str) -> String {
    if address.len() <= 10 {
        return address.to_string();
    }
    format!("{}...{}", &address[..6], &address[address.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let password = "test_password";
        let encrypted = encrypt_key(key, password).unwrap();
        let decrypted = decrypt_key(&encrypted, password).unwrap();
        assert_eq!(decrypted, key);
    }

    #[test]
    fn test_decrypt_wrong_password() {
        let key = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let encrypted = encrypt_key(key, "correct").unwrap();
        assert!(decrypt_key(&encrypted, "wrong").is_err());
    }

    #[test]
    fn test_format_address() {
        assert_eq!(
            format_address("0x1234567890abcdef"),
            "0x1234...cdef"
        );
    }

    #[test]
    fn test_format_address_short() {
        assert_eq!(format_address("0x1234"), "0x1234");
    }
}
