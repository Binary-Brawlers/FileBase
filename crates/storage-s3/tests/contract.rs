//! Runs the shared storage contract against an S3-compatible server (moto in CI).
//! Skipped unless `FILEBASE_TEST_S3_ENDPOINT` is set. The bucket must exist.

use filebase_storage::contract::run_adapter_contract;
use filebase_storage_s3::{S3Config, S3StorageAdapter};

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => value,
        _ => default.to_string(),
    }
}

#[tokio::test]
async fn s3_adapter_satisfies_storage_contract() {
    let endpoint = match std::env::var("FILEBASE_TEST_S3_ENDPOINT") {
        Ok(endpoint) if !endpoint.is_empty() => endpoint,
        _ => {
            eprintln!("skipping: FILEBASE_TEST_S3_ENDPOINT is not set");
            return;
        }
    };
    let public_base_url = "https://s3-files.example.test";
    let adapter = S3StorageAdapter::new(S3Config {
        bucket: env_or("FILEBASE_TEST_S3_BUCKET", "filebase-test"),
        region: env_or("FILEBASE_TEST_S3_REGION", "us-east-1"),
        endpoint: Some(endpoint),
        access_key: env_or("FILEBASE_TEST_S3_ACCESS_KEY", "minioadmin"),
        secret_key: env_or("FILEBASE_TEST_S3_SECRET_KEY", "minioadmin"),
        force_path_style: true,
        prefix: "contract-prefix".into(),
        public_base_url: public_base_url.into(),
    });
    let prefix = format!("contract-{}", std::process::id());
    run_adapter_contract(&adapter, &prefix, public_base_url).await;
}
