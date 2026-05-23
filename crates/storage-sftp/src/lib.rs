use std::{io::Write, net::TcpStream, path::Path};

use async_trait::async_trait;
use filebase_storage::{
    join_path, join_url, StorageAdapter, StorageError, StorageResult, UploadInput, UploadResult,
};
use ssh2::{Session, Sftp};
use tokio::task;

#[derive(Clone)]
pub struct SftpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub base_path: String,
    pub public_base_url: String,
}

pub struct SftpStorageAdapter {
    config: SftpConfig,
}

impl SftpStorageAdapter {
    pub fn new(config: SftpConfig) -> Self {
        Self { config }
    }
}

fn connect(cfg: &SftpConfig) -> StorageResult<(Session, Sftp)> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let tcp = TcpStream::connect(&addr)
        .map_err(|e| StorageError::Backend(format!("sftp tcp connect: {e}")))?;
    let mut session =
        Session::new().map_err(|e| StorageError::Backend(format!("sftp session: {e}")))?;
    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| StorageError::Backend(format!("sftp handshake: {e}")))?;

    if let Some(key) = cfg.private_key.as_deref() {
        session
            .userauth_pubkey_memory(&cfg.username, None, key, cfg.password.as_deref())
            .map_err(|e| StorageError::Authentication(format!("sftp pubkey: {e}")))?;
    } else if let Some(pw) = cfg.password.as_deref() {
        session
            .userauth_password(&cfg.username, pw)
            .map_err(|e| StorageError::Authentication(format!("sftp password: {e}")))?;
    } else {
        return Err(StorageError::Configuration(
            "sftp requires password or private_key".into(),
        ));
    }

    if !session.authenticated() {
        return Err(StorageError::Authentication(
            "sftp not authenticated".into(),
        ));
    }

    let sftp = session
        .sftp()
        .map_err(|e| StorageError::Backend(format!("sftp open: {e}")))?;
    Ok((session, sftp))
}

fn ensure_dirs(sftp: &Sftp, dir: &Path) -> StorageResult<()> {
    if dir.as_os_str().is_empty() {
        return Ok(());
    }
    if sftp.stat(dir).is_ok() {
        return Ok(());
    }
    if let Some(parent) = dir.parent() {
        if !parent.as_os_str().is_empty() {
            ensure_dirs(sftp, parent)?;
        }
    }
    sftp.mkdir(dir, 0o755)
        .map_err(|e| StorageError::Backend(format!("sftp mkdir {dir:?}: {e}")))?;
    Ok(())
}

#[async_trait]
impl StorageAdapter for SftpStorageAdapter {
    async fn upload(&self, input: UploadInput) -> StorageResult<UploadResult> {
        let cfg = self.config.clone();
        let rel_path = input.path.clone();
        let bytes = input.bytes;
        let size = bytes.len() as u64;

        task::spawn_blocking(move || -> StorageResult<()> {
            let (_session, sftp) = connect(&cfg)?;
            let full = join_path(&cfg.base_path, &rel_path);
            let full_path = std::path::PathBuf::from(&full);
            if let Some(parent) = full_path.parent() {
                ensure_dirs(&sftp, parent)?;
            }
            let mut file = sftp
                .create(&full_path)
                .map_err(|e| StorageError::Backend(format!("sftp create: {e}")))?;
            file.write_all(&bytes)
                .map_err(|e| StorageError::Io(format!("sftp write: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Backend(format!("sftp task join: {e}")))??;

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
            let (_session, sftp) = connect(&cfg)?;
            let full = std::path::PathBuf::from(join_path(&cfg.base_path, &path));
            match sftp.unlink(&full) {
                Ok(_) => Ok(()),
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("no such file") || msg.contains("not found") {
                        Ok(())
                    } else {
                        Err(StorageError::Backend(format!("sftp unlink: {e}")))
                    }
                }
            }
        })
        .await
        .map_err(|e| StorageError::Backend(format!("sftp task join: {e}")))?
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let cfg = self.config.clone();
        let path = path.to_string();
        task::spawn_blocking(move || -> StorageResult<bool> {
            let (_session, sftp) = connect(&cfg)?;
            let full = std::path::PathBuf::from(join_path(&cfg.base_path, &path));
            Ok(sftp.stat(&full).is_ok())
        })
        .await
        .map_err(|e| StorageError::Backend(format!("sftp task join: {e}")))?
    }

    async fn public_url(&self, path: &str) -> StorageResult<String> {
        Ok(join_url(&self.config.public_base_url, path))
    }

    async fn health_check(&self) -> StorageResult<()> {
        let cfg = self.config.clone();
        task::spawn_blocking(move || -> StorageResult<()> {
            let (_session, sftp) = connect(&cfg)?;
            let base = std::path::PathBuf::from(&cfg.base_path);
            ensure_dirs(&sftp, &base)?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::Backend(format!("sftp task join: {e}")))?
    }
}
