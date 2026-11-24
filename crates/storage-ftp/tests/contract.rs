//! Runs the shared storage contract against a real FTP server.
//! Skipped unless `FILEBASE_TEST_FTP_HOST` is set (CI starts a server).

use filebase_storage::contract::run_adapter_contract;
use filebase_storage_ftp::{FtpConfig, FtpStorageAdapter};

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => value,
        _ => default.to_string(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ftp_adapter_satisfies_storage_contract() {
    let host = match std::env::var("FILEBASE_TEST_FTP_HOST") {
        Ok(host) if !host.is_empty() => host,
        _ => {
            eprintln!("skipping: FILEBASE_TEST_FTP_HOST is not set");
            return;
        }
    };
    let port = match env_or("FILEBASE_TEST_FTP_PORT", "21").parse::<u16>() {
        Ok(port) => port,
        Err(error) => panic!("FILEBASE_TEST_FTP_PORT is invalid: {error}"),
    };
    let public_base_url = "https://ftp-files.example.test";
    let adapter = FtpStorageAdapter::new(FtpConfig {
        host,
        port,
        username: env_or("FILEBASE_TEST_FTP_USER", "filebase"),
        password: env_or("FILEBASE_TEST_FTP_PASSWORD", "filebase"),
        base_path: env_or("FILEBASE_TEST_FTP_BASE_PATH", "/"),
        public_base_url: public_base_url.into(),
    });
    let prefix = format!("contract-{}", std::process::id());
    run_adapter_contract(&adapter, &prefix, public_base_url).await;
}
