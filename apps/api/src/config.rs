use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageDriver {
    Local,
    Ftp,
    Sftp,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateStrategy {
    ReturnExisting,
    UploadNewCopy,
    RejectDuplicate,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub app_url: String,
    pub dashboard_url: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub database_connect_timeout_seconds: u64,
    pub redis_url: String,
    pub jwt_secret: String,
    pub encryption_key: String,
    pub storage_driver: StorageDriver,
    pub local_storage_path: String,
    pub public_base_url: String,
    pub max_upload_size: u64,
    pub auth_rate_limit_per_minute: usize,
    pub upload_rate_limit_per_minute: usize,
    pub default_duplicate_strategy: DuplicateStrategy,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    Invalid(&'static str, String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address = optional("BIND_ADDRESS")
            .unwrap_or_else(|| "0.0.0.0:8080".to_string())
            .parse()
            .map_err(|e: std::net::AddrParseError| {
                ConfigError::Invalid("BIND_ADDRESS", e.to_string())
            })?;

        let app_url = required("APP_URL")?;
        let dashboard_url = required("DASHBOARD_URL")?;
        let database_url = required("DATABASE_URL")?;
        let database_max_connections = optional("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|| "10".to_string())
            .parse::<u32>()
            .map_err(|e| ConfigError::Invalid("DATABASE_MAX_CONNECTIONS", e.to_string()))?;
        let database_min_connections = optional("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|| "1".to_string())
            .parse::<u32>()
            .map_err(|e| ConfigError::Invalid("DATABASE_MIN_CONNECTIONS", e.to_string()))?;
        let database_connect_timeout_seconds = optional("DATABASE_CONNECT_TIMEOUT_SECONDS")
            .unwrap_or_else(|| "5".to_string())
            .parse::<u64>()
            .map_err(|e| ConfigError::Invalid("DATABASE_CONNECT_TIMEOUT_SECONDS", e.to_string()))?;
        let redis_url = required("REDIS_URL")?;
        let jwt_secret = required("JWT_SECRET")?;
        let encryption_key = required("ENCRYPTION_KEY")?;

        let storage_driver = match required("STORAGE_DRIVER")?.as_str() {
            "local" => StorageDriver::Local,
            "ftp" => StorageDriver::Ftp,
            "sftp" => StorageDriver::Sftp,
            other => return Err(ConfigError::Invalid("STORAGE_DRIVER", other.to_string())),
        };

        let local_storage_path =
            optional("LOCAL_STORAGE_PATH").unwrap_or_else(|| "./uploads".to_string());
        let public_base_url = required("PUBLIC_BASE_URL")?;

        let max_upload_size = optional("MAX_UPLOAD_SIZE")
            .unwrap_or_else(|| "10485760".to_string())
            .parse::<u64>()
            .map_err(|e| ConfigError::Invalid("MAX_UPLOAD_SIZE", e.to_string()))?;
        let auth_rate_limit_per_minute = optional("AUTH_RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|| "10".to_string())
            .parse::<usize>()
            .map_err(|e| ConfigError::Invalid("AUTH_RATE_LIMIT_PER_MINUTE", e.to_string()))?;
        let upload_rate_limit_per_minute = optional("UPLOAD_RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|| "60".to_string())
            .parse::<usize>()
            .map_err(|e| ConfigError::Invalid("UPLOAD_RATE_LIMIT_PER_MINUTE", e.to_string()))?;

        let default_duplicate_strategy = match optional("DEFAULT_DUPLICATE_STRATEGY")
            .unwrap_or_else(|| "return_existing".to_string())
            .as_str()
        {
            "return_existing" => DuplicateStrategy::ReturnExisting,
            "upload_new_copy" => DuplicateStrategy::UploadNewCopy,
            "reject_duplicate" => DuplicateStrategy::RejectDuplicate,
            other => {
                return Err(ConfigError::Invalid(
                    "DEFAULT_DUPLICATE_STRATEGY",
                    other.to_string(),
                ))
            }
        };

        Ok(Self {
            bind_address,
            app_url,
            dashboard_url,
            database_url,
            database_max_connections,
            database_min_connections,
            database_connect_timeout_seconds,
            redis_url,
            jwt_secret,
            encryption_key,
            storage_driver,
            local_storage_path,
            public_base_url,
            max_upload_size,
            auth_rate_limit_per_minute,
            upload_rate_limit_per_minute,
            default_duplicate_strategy,
        })
    }
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    std::env::var(key).map_err(|_| ConfigError::Missing(key))
}

fn optional(key: &'static str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
