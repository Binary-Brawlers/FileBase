use async_trait::async_trait;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage misconfigured: {0}")]
    Configuration(String),
    #[error("storage authentication failed: {0}")]
    Authentication(String),
    #[error("storage io error: {0}")]
    Io(String),
    #[error("storage backend error: {0}")]
    Backend(String),
}

#[derive(Debug, Clone)]
pub struct UploadInput {
    /// Path relative to the adapter's base path.
    pub path: String,
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UploadResult {
    pub path: String,
    pub url: String,
    pub size: u64,
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    async fn upload(&self, input: UploadInput) -> StorageResult<UploadResult>;
    async fn delete(&self, path: &str) -> StorageResult<()>;
    async fn exists(&self, path: &str) -> StorageResult<bool>;
    async fn public_url(&self, path: &str) -> StorageResult<String>;
    /// Verify the connection is usable. Used by the connection-test endpoint.
    async fn health_check(&self) -> StorageResult<()>;
}

/// Join a base URL and a relative path with exactly one `/` between them.
pub fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

/// Join a base path and a relative path, normalising slashes for remote backends.
pub fn join_path(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if base.is_empty() {
        path.to_string()
    } else {
        format!("{base}/{path}")
    }
}
