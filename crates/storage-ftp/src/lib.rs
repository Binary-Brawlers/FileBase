use std::io::Cursor;

use async_trait::async_trait;
use filebase_storage::{
    join_path, join_url, StorageAdapter, StorageError, StorageResult, UploadInput, UploadResult,
};
use suppaftp::FtpStream;
use tokio::task;

#[derive(Clone)]
pub struct FtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub base_path: String,
    pub public_base_url: String,
}

pub struct FtpStorageAdapter {
    config: FtpConfig,
}

impl FtpStorageAdapter {
    pub fn new(config: FtpConfig) -> Self {
        Self { config }
    }

    fn connect(&self) -> StorageResult<FtpStream> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let mut stream = FtpStream::connect(&addr)
            .map_err(|e| StorageError::Backend(format!("ftp connect: {e}")))?;
        stream
            .login(&self.config.username, &self.config.password)
            .map_err(|e| StorageError::Authentication(format!("ftp login: {e}")))?;
        Ok(stream)
    }
}

fn ensure_dirs(stream: &mut FtpStream, dir: &str) -> StorageResult<()> {
    if dir.is_empty() || dir == "/" {
        return Ok(());
    }
    let mut cursor = String::new();
    for part in dir.split('/').filter(|p| !p.is_empty()) {
        cursor.push('/');
        cursor.push_str(part);
        if stream.cwd(&cursor).is_err() {
            stream
                .mkdir(&cursor)
                .map_err(|e| StorageError::Backend(format!("ftp mkdir {cursor}: {e}")))?;
        }
    }
    Ok(())
}

#[async_trait]
impl StorageAdapter for FtpStorageAdapter {
    async fn upload(&self, input: UploadInput) -> StorageResult<UploadResult> {
        let cfg = self.config.clone();
        let path = input.path.clone();
        let bytes = input.bytes;
        let size = bytes.len() as u64;

        task::spawn_blocking(move || -> StorageResult<()> {
            let mut stream = FtpStorageAdapter::new(cfg.clone()).connect()?;
            let full = join_path(&cfg.base_path, &path);
            let parent = match full.rsplit_once('/') {
                Some((p, _)) => p.to_string(),
                None => String::new(),
            };
            ensure_dirs(&mut stream, &parent)?;
            let mut reader = Cursor::new(bytes);
            stream
                .put_file(&full, &mut reader)
                .map(|_| ())
                .map_err(|e| StorageError::Backend(format!("ftp put_file: {e}")))?;
            let _ = stream.quit();
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Backend(format!("ftp task join: {e}")))??;

        Ok(UploadResult {
            url: join_url(&self.config.public_base_url, &input.path),
            path: input.path,
            size,
        })
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        let cfg = self.config.clone();
        let path = path.to_string();
        task::spawn_blocking(move || -> StorageResult<()> {
            let mut stream = FtpStorageAdapter::new(cfg.clone()).connect()?;
            let full = join_path(&cfg.base_path, &path);
            match stream.rm(&full) {
                Ok(_) => {}
                Err(e) => {
                    let msg = e.to_string();
                    if !msg.contains("550") {
                        return Err(StorageError::Backend(format!("ftp rm: {e}")));
                    }
                }
            }
            let _ = stream.quit();
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Backend(format!("ftp task join: {e}")))?
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let cfg = self.config.clone();
        let path = path.to_string();
        task::spawn_blocking(move || -> StorageResult<bool> {
            let mut stream = FtpStorageAdapter::new(cfg.clone()).connect()?;
            let full = join_path(&cfg.base_path, &path);
            let result = stream.size(&full).is_ok();
            let _ = stream.quit();
            Ok(result)
        })
        .await
        .map_err(|e| StorageError::Backend(format!("ftp task join: {e}")))?
    }

    async fn public_url(&self, path: &str) -> StorageResult<String> {
        Ok(join_url(&self.config.public_base_url, path))
    }

    async fn health_check(&self) -> StorageResult<()> {
        let cfg = self.config.clone();
        task::spawn_blocking(move || -> StorageResult<()> {
            let mut stream = FtpStorageAdapter::new(cfg.clone()).connect()?;
            stream
                .cwd(&cfg.base_path)
                .map_err(|e| StorageError::Backend(format!("ftp cwd {}: {e}", cfg.base_path)))?;
            let _ = stream.quit();
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Backend(format!("ftp task join: {e}")))?
    }
}
