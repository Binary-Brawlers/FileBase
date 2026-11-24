//! Runs the shared storage contract against a real SFTP server.
//! Skipped unless `FILEBASE_TEST_SFTP_HOST` is set (CI starts a server).
//! Authenticates with `FILEBASE_TEST_SFTP_PRIVATE_KEY_FILE` when set,
//! otherwise with `FILEBASE_TEST_SFTP_PASSWORD`.

use filebase_storage::contract::run_adapter_contract;
use filebase_storage_sftp::{SftpConfig, SftpStorageAdapter};

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => value,
        _ => default.to_string(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sftp_adapter_satisfies_storage_contract() {
    let host = match std::env::var("FILEBASE_TEST_SFTP_HOST") {
        Ok(host) if !host.is_empty() => host,
        _ => {
            eprintln!("skipping: FILEBASE_TEST_SFTP_HOST is not set");
            return;
        }
    };
    let port = match env_or("FILEBASE_TEST_SFTP_PORT", "22").parse::<u16>() {
        Ok(port) => port,
        Err(error) => panic!("FILEBASE_TEST_SFTP_PORT is invalid: {error}"),
    };
    let private_key = match std::env::var("FILEBASE_TEST_SFTP_PRIVATE_KEY_FILE") {
        Ok(path) if !path.is_empty() => match std::fs::read_to_string(&path) {
            Ok(key) => Some(key),
            Err(error) => panic!("cannot read {path}: {error}"),
        },
        _ => None,
    };
    let password = match private_key {
        Some(_) => None,
        None => Some(env_or("FILEBASE_TEST_SFTP_PASSWORD", "filebase")),
    };
    let public_base_url = "https://sftp-files.example.test";
    let adapter = SftpStorageAdapter::new(SftpConfig {
        host,
        port,
        username: env_or("FILEBASE_TEST_SFTP_USER", "filebase"),
        password,
        private_key,
        base_path: env_or("FILEBASE_TEST_SFTP_BASE_PATH", "upload"),
        public_base_url: public_base_url.into(),
    });
    let prefix = filebase_storage::contract::unique_prefix();
    run_adapter_contract(&adapter, &prefix, public_base_url).await;
}
