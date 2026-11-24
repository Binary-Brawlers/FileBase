use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

pub const TOKEN_TTL_HOURS: i64 = 24 * 7;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_token(secret: &str, user_id: &str, email: &str) -> Result<String, ApiError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp: (now + Duration::hours(TOKEN_TTL_HOURS)).timestamp(),
        iat: now.timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))
}

pub fn decode_token(secret: &str, token: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-jwt-secret";

    #[test]
    fn issued_token_decodes_to_the_same_identity() {
        let token = match issue_token(SECRET, "usr_1", "admin@example.test") {
            Ok(token) => token,
            Err(error) => panic!("issue failed: {error}"),
        };
        match decode_token(SECRET, &token) {
            Ok(claims) => {
                assert_eq!(claims.sub, "usr_1");
                assert_eq!(claims.email, "admin@example.test");
                assert_eq!(claims.exp - claims.iat, TOKEN_TTL_HOURS * 3600);
            }
            Err(error) => panic!("decode failed: {error}"),
        }
    }

    #[test]
    fn token_signed_with_another_secret_is_unauthorized() {
        let token = match issue_token("other-secret", "usr_1", "admin@example.test") {
            Ok(token) => token,
            Err(error) => panic!("issue failed: {error}"),
        };
        assert!(matches!(
            decode_token(SECRET, &token),
            Err(ApiError::Unauthorized)
        ));
    }

    #[test]
    fn expired_token_is_unauthorized() {
        let issued_at = Utc::now() - Duration::hours(2);
        let claims = Claims {
            sub: "usr_1".into(),
            email: "admin@example.test".into(),
            exp: (issued_at + Duration::hours(1)).timestamp(),
            iat: issued_at.timestamp(),
        };
        let token = match encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        ) {
            Ok(token) => token,
            Err(error) => panic!("encode failed: {error}"),
        };
        assert!(matches!(
            decode_token(SECRET, &token),
            Err(ApiError::Unauthorized)
        ));
    }

    #[test]
    fn malformed_token_is_unauthorized() {
        assert!(matches!(
            decode_token(SECRET, "not.a.jwt"),
            Err(ApiError::Unauthorized)
        ));
    }
}
