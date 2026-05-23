use async_trait::async_trait;
use filebase_core::FileBaseResult;

pub struct UploadInput {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub struct UploadResult {
    pub path: String,
    pub url: String,
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    async fn upload(&self, input: UploadInput) -> FileBaseResult<UploadResult>;
    async fn delete(&self, path: &str) -> FileBaseResult<()>;
    async fn exists(&self, path: &str) -> FileBaseResult<bool>;
    async fn public_url(&self, path: &str) -> FileBaseResult<String>;
}
