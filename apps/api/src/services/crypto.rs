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
        return Err(ApiError::Internal(anyhow::anyhow!("decrypt: short payload")));
    }
    let (nonce_bytes, ciphertext) = bytes.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher(key)
        .decrypt(nonce, ciphertext)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("decrypt: {e}")))?;
    String::from_utf8(plaintext)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("decrypt utf8: {e}")))
}
