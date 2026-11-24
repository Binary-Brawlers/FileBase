use crate::error::ApiError;

const BCRYPT_COST: u32 = 12;

pub fn hash(password: &str) -> Result<String, ApiError> {
    bcrypt::hash(password, BCRYPT_COST).map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))
}

pub fn verify(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashed_password_verifies_only_with_the_original() {
        let hashed = match hash("correct horse battery") {
            Ok(hashed) => hashed,
            Err(error) => panic!("hash failed: {error}"),
        };
        assert_ne!(hashed, "correct horse battery");
        assert!(verify("correct horse battery", &hashed));
        assert!(!verify("wrong password", &hashed));
    }

    #[test]
    fn malformed_hash_never_verifies() {
        assert!(!verify("anything", "not-a-bcrypt-hash"));
    }
}
