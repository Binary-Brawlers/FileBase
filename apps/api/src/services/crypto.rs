use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha2::{Digest, Sha256};

use crate::error::ApiError;

fn cipher(key: &str) -> Aes256Gcm {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let derived = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&derived);
    Aes256Gcm::new(key)
}

pub fn encrypt(plaintext: &str, key: &str) -> Result<String, ApiError> {
    let cipher = cipher(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("encrypt: {e}")))?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ciphertext);
    Ok(B64.encode(out))
}

#[allow(dead_code)]
pub fn decrypt(payload: &str, key: &str) -> Result<String, ApiError> {
    let bytes = B64
        .decode(payload)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("decrypt base64: {e}")))?;
    if bytes.len() < 12 {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "decrypt: short payload"
        )));
    }
    let (nonce_bytes, ciphertext) = bytes.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher(key)
        .decrypt(nonce, ciphertext)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("decrypt: {e}")))?;
    String::from_utf8(plaintext)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("decrypt utf8: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "test-encryption-key";

    #[test]
    fn encrypt_then_decrypt_returns_original_plaintext() {
        let encrypted = match encrypt("ftp-password", KEY) {
            Ok(value) => value,
            Err(error) => panic!("encrypt failed: {error}"),
        };
        assert_ne!(encrypted, "ftp-password");
        match decrypt(&encrypted, KEY) {
            Ok(plaintext) => assert_eq!(plaintext, "ftp-password"),
            Err(error) => panic!("decrypt failed: {error}"),
        }
    }

    #[test]
    fn encrypting_twice_uses_a_fresh_nonce() {
        let first = encrypt("same", KEY);
        let second = encrypt("same", KEY);
        match (first, second) {
            (Ok(first), Ok(second)) => assert_ne!(first, second),
            _ => panic!("encrypt failed"),
        }
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let encrypted = match encrypt("secret", KEY) {
            Ok(value) => value,
            Err(error) => panic!("encrypt failed: {error}"),
        };
        assert!(decrypt(&encrypted, "another-key").is_err());
    }

    #[test]
    fn decrypt_rejects_tampered_ciphertext() {
        let encrypted = match encrypt("secret", KEY) {
            Ok(value) => value,
            Err(error) => panic!("encrypt failed: {error}"),
        };
        let mut bytes = match B64.decode(encrypted) {
            Ok(bytes) => bytes,
            Err(error) => panic!("base64 decode failed: {error}"),
        };
        if let Some(last) = bytes.last_mut() {
            *last ^= 0x01;
        }
        assert!(decrypt(&B64.encode(bytes), KEY).is_err());
    }

    #[test]
    fn decrypt_rejects_short_and_non_base64_payloads() {
        assert!(decrypt(&B64.encode([0_u8; 4]), KEY).is_err());
        assert!(decrypt("not base64 !!", KEY).is_err());
    }
}
