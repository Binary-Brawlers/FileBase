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
) -> Result<Arc<dyn StorageAdapter>, ApiError> {
    match model.r#type.as_str() {
        "local" => Ok(Arc::new(LocalStorageAdapter::new(
            model.base_path.clone(),
            model.public_base_url.clone(),
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
                public_base_url: model.public_base_url.clone(),
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
                public_base_url: model.public_base_url.clone(),
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
                public_base_url: model.public_base_url.clone(),
            })))
        }
        other => Err(ApiError::Validation(format!(
            "unknown storage type: {other}"
        ))),
    }
}
