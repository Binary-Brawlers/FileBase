use crate::error::ApiError;

const BCRYPT_COST: u32 = 12;

pub fn hash(password: &str) -> Result<String, ApiError> {
    bcrypt::hash(password, BCRYPT_COST).map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))
}

pub fn verify(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}
