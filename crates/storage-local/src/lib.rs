use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use filebase_storage::{
    join_url, StorageAdapter, StorageError, StorageResult, UploadInput, UploadResult,
};
use tokio::{fs, io::AsyncWriteExt};

pub struct LocalStorageAdapter {
    base_path: PathBuf,
    public_base_url: String,
}

impl LocalStorageAdapter {
    pub fn new(base_path: impl Into<PathBuf>, public_base_url: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            public_base_url: public_base_url.into(),
        }
    }

    fn resolve(&self, rel: &str) -> StorageResult<PathBuf> {
        let candidate = Path::new(rel);
        for c in candidate.components() {
            match c {
                Component::Normal(_) => {}
                _ => {
                    return Err(StorageError::Configuration(format!(
                        "invalid path component in {rel:?}"
                    )))
                }
            }
        }
        Ok(self.base_path.join(candidate))
    }
}

fn io_err(err: std::io::Error) -> StorageError {
    StorageError::Io(err.to_string())
}

#[async_trait]
impl StorageAdapter for LocalStorageAdapter {
    async fn upload(&self, input: UploadInput) -> StorageResult<UploadResult> {
        let target = self.resolve(&input.path)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await.map_err(io_err)?;
        }
        let mut file = fs::File::create(&target).await.map_err(io_err)?;
        file.write_all(&input.bytes).await.map_err(io_err)?;
        file.flush().await.map_err(io_err)?;
        let size = input.bytes.len() as u64;
        Ok(UploadResult {
            url: join_url(&self.public_base_url, &input.path),
            path: input.path,
            size,
        })
    }

    async fn download(&self, path: &str) -> StorageResult<Vec<u8>> {
        let target = self.resolve(path)?;
        fs::read(&target).await.map_err(io_err)
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        let target = self.resolve(path)?;
        match fs::remove_file(&target).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_err(e)),
        }
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let target = self.resolve(path)?;
        match fs::metadata(&target).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(io_err(e)),
        }
    }

    async fn public_url(&self, path: &str) -> StorageResult<String> {
        Ok(join_url(&self.public_base_url, path))
    }

    async fn health_check(&self) -> StorageResult<()> {
        fs::create_dir_all(&self.base_path).await.map_err(io_err)?;
        let probe = self.base_path.join(".filebase_write_test");
        fs::write(&probe, b"ok").await.map_err(io_err)?;
        fs::remove_file(&probe).await.map_err(io_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filebase_storage::contract::run_adapter_contract;

    fn unique_temp_dir() -> PathBuf {
        let nanos = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(elapsed) => elapsed.as_nanos(),
            Err(_) => 0,
        };
        std::env::temp_dir().join(format!(
            "filebase-local-test-{}-{nanos}",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn local_adapter_satisfies_storage_contract() {
        let base = unique_temp_dir();
        let adapter = LocalStorageAdapter::new(base.clone(), "https://files.example.test/");
        run_adapter_contract(&adapter, "contract", "https://files.example.test/").await;
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn rejects_paths_that_escape_the_base_directory() {
        let base = unique_temp_dir();
        let adapter = LocalStorageAdapter::new(base.clone(), "https://files.example.test");
        for path in ["../escape.txt", "/etc/passwd", "a/../../b.txt", "./a.txt"] {
            let result = adapter
                .upload(UploadInput {
                    path: path.into(),
                    bytes: b"x".to_vec(),
                    content_type: None,
                })
                .await;
            assert!(
                matches!(result, Err(StorageError::Configuration(_))),
                "{path} should be rejected"
            );
            assert!(adapter.download(path).await.is_err());
            assert!(adapter.delete(path).await.is_err());
        }
        let _ = std::fs::remove_dir_all(&base);
    }
}
