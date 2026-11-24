use std::sync::Arc;

use filebase_storage::StorageAdapter;
use filebase_storage_ftp::{FtpConfig, FtpStorageAdapter};
use filebase_storage_local::LocalStorageAdapter;
use filebase_storage_s3::{S3Config, S3StorageAdapter};
use filebase_storage_sftp::{SftpConfig, SftpStorageAdapter};

use crate::entities::storage_connection;
use crate::error::ApiError;
use crate::services::crypto;

pub fn build_adapter(
    model: &storage_connection::Model,
    encryption_key: &str,
    cdn_base_url: Option<&str>,
) -> Result<Arc<dyn StorageAdapter>, ApiError> {
    let public_base_url = cdn_base_url
        .map(|base| base.trim_end_matches('/').to_string())
        .unwrap_or_else(|| model.public_base_url.clone());
    match model.r#type.as_str() {
        "local" => Ok(Arc::new(LocalStorageAdapter::new(
            model.base_path.clone(),
            public_base_url.clone(),
        ))),
        "ftp" => {
            let host = model
                .host
                .clone()
                .ok_or_else(|| ApiError::Validation("ftp host missing".into()))?;
            let username = model
                .username
                .clone()
                .ok_or_else(|| ApiError::Validation("ftp username missing".into()))?;
            let encrypted = model
                .encrypted_password
                .as_deref()
                .ok_or_else(|| ApiError::Validation("ftp password missing".into()))?;
            let password = crypto::decrypt(encrypted, encryption_key)?;
            Ok(Arc::new(FtpStorageAdapter::new(FtpConfig {
                host,
                port: model.port.unwrap_or(21) as u16,
                username,
                password,
                base_path: model.base_path.clone(),
                public_base_url: public_base_url.clone(),
            })))
        }
        "sftp" => {
            let host = model
                .host
                .clone()
                .ok_or_else(|| ApiError::Validation("sftp host missing".into()))?;
            let username = model
                .username
                .clone()
                .ok_or_else(|| ApiError::Validation("sftp username missing".into()))?;
            let password = match model.encrypted_password.as_deref() {
                Some(p) => Some(crypto::decrypt(p, encryption_key)?),
                None => None,
            };
            let private_key = match model.encrypted_private_key.as_deref() {
                Some(k) => Some(crypto::decrypt(k, encryption_key)?),
                None => None,
            };
            Ok(Arc::new(SftpStorageAdapter::new(SftpConfig {
                host,
                port: model.port.unwrap_or(22) as u16,
                username,
                password,
                private_key,
                base_path: model.base_path.clone(),
                public_base_url: public_base_url.clone(),
            })))
        }
        "s3" => {
            let bucket = model
                .bucket
                .clone()
                .ok_or_else(|| ApiError::Validation("s3 bucket missing".into()))?;
            let region = model
                .region
                .clone()
                .ok_or_else(|| ApiError::Validation("s3 region missing".into()))?;
            let access_key = model
                .username
                .clone()
                .ok_or_else(|| ApiError::Validation("s3 access key missing".into()))?;
            let encrypted = model
                .encrypted_password
                .as_deref()
                .ok_or_else(|| ApiError::Validation("s3 secret key missing".into()))?;
            let secret_key = crypto::decrypt(encrypted, encryption_key)?;
            let endpoint = model.host.clone().filter(|h| !h.is_empty());
            Ok(Arc::new(S3StorageAdapter::new(S3Config {
                bucket,
                region,
                endpoint,
                access_key,
                secret_key,
                force_path_style: model.force_path_style,
                prefix: model.base_path.clone(),
                public_base_url,
            })))
        }
        other => Err(ApiError::Validation(format!(
            "unknown storage type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "test-encryption-key";

    fn connection(kind: &str) -> storage_connection::Model {
        let now = chrono::Utc::now().fixed_offset();
        storage_connection::Model {
            id: "stc_1".into(),
            project_id: "prj_1".into(),
            r#type: kind.into(),
            host: None,
            port: None,
            username: None,
            encrypted_password: None,
            encrypted_private_key: None,
            bucket: None,
            region: None,
            force_path_style: false,
            base_path: "/srv/uploads".into(),
            public_base_url: "https://files.example.test/".into(),
            created_at: now,
            updated_at: now,
        }
    }

    fn validation_message(result: Result<Arc<dyn StorageAdapter>, ApiError>) -> String {
        match result {
            Err(ApiError::Validation(message)) => message,
            Err(other) => panic!("expected validation error, got {other}"),
            Ok(_) => panic!("expected validation error, got an adapter"),
        }
    }

    #[tokio::test]
    async fn local_adapter_prefers_cdn_base_url_for_public_urls() {
        let adapter =
            match build_adapter(&connection("local"), KEY, Some("https://cdn.example.test/")) {
                Ok(adapter) => adapter,
                Err(error) => panic!("build failed: {error}"),
            };
        match adapter.public_url("images/a.png").await {
            Ok(url) => assert_eq!(url, "https://cdn.example.test/images/a.png"),
            Err(error) => panic!("public_url failed: {error}"),
        }
    }

    #[test]
    fn remote_adapters_require_their_connection_fields() {
        assert_eq!(
            validation_message(build_adapter(&connection("ftp"), KEY, None)),
            "ftp host missing"
        );
        assert_eq!(
            validation_message(build_adapter(&connection("sftp"), KEY, None)),
            "sftp host missing"
        );
        assert_eq!(
            validation_message(build_adapter(&connection("s3"), KEY, None)),
            "s3 bucket missing"
        );
    }

    #[test]
    fn unknown_storage_type_is_rejected() {
        assert_eq!(
            validation_message(build_adapter(&connection("dropbox"), KEY, None)),
            "unknown storage type: dropbox"
        );
    }

    #[test]
    fn credentials_encrypted_with_another_key_fail_to_load() {
        let mut model = connection("ftp");
        model.host = Some("ftp.example.test".into());
        model.username = Some("uploader".into());
        model.encrypted_password = match crypto::encrypt("secret", "different-key") {
            Ok(value) => Some(value),
            Err(error) => panic!("encrypt failed: {error}"),
        };
        assert!(matches!(
            build_adapter(&model, KEY, None),
            Err(ApiError::Internal(_))
        ));
    }
}
