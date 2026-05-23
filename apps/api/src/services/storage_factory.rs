use std::sync::Arc;

use filebase_storage::StorageAdapter;
use filebase_storage_ftp::{FtpConfig, FtpStorageAdapter};
use filebase_storage_local::LocalStorageAdapter;
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
        other => Err(ApiError::Validation(format!(
            "unknown storage type: {other}"
        ))),
    }
}
