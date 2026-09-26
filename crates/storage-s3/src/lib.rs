use async_trait::async_trait;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use filebase_storage::{
    join_path, join_url, StorageAdapter, StorageError, StorageResult, UploadInput, UploadResult,
};

/// Credentials for an S3-compatible object store.
///
/// Reuses the shared `storage_connections` column layout: the access key is
/// stored as `username`, the secret key as `encrypted_password`, an optional
/// custom endpoint as `host`, the bucket as `bucket`, the region as `region`,
/// and an optional object key prefix as `base_path`.
#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    /// Custom endpoint (e.g. Cloudflare R2, DigitalOcean Spaces, Backblaze B2,
    /// Wasabi). When `None`, the AWS default endpoint for `region` is used.
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    /// Some S3-compatible providers require path-style addressing.
    pub force_path_style: bool,
    /// Optional key prefix, equivalent to the FTP/SFTP "base path".
    pub prefix: String,
    /// Base URL used to build public file URLs.
    pub public_base_url: String,
}

pub struct S3StorageAdapter {
    config: S3Config,
    client: Client,
}

impl S3StorageAdapter {
    pub fn new(config: S3Config) -> Self {
        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(Credentials::new(
                config.access_key.clone(),
                config.secret_key.clone(),
                None,
                None,
                "static",
            ));
        if let Some(endpoint) = config.endpoint.as_deref() {
            builder = builder.endpoint_url(endpoint);
        }
        builder = builder.force_path_style(config.force_path_style);
        let client = Client::from_conf(builder.build());
        Self { config, client }
    }

    fn key(&self, path: &str) -> String {
        if self.config.prefix.is_empty() {
            path.to_string()
        } else {
            join_path(&self.config.prefix, path)
        }
    }
}

fn map_err<E>(op: &str, err: E) -> StorageError
where
    E: std::fmt::Display,
{
    StorageError::Backend(format!("s3 {op}: {err}"))
}

/// True when the S3 response indicates a missing resource (HTTP 404).
fn is_missing<E>(e: &aws_sdk_s3::error::SdkError<E>) -> bool {
    e.raw_response()
        .map(|r| r.status().as_u16() == 404)
        .unwrap_or(false)
}

#[async_trait]
impl StorageAdapter for S3StorageAdapter {
    async fn upload(&self, input: UploadInput) -> StorageResult<UploadResult> {
        let key = self.key(&input.path);
        let size = input.bytes.len() as u64;
        let mut req = self
            .client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(ByteStream::from(input.bytes));
        if let Some(ct) = input.content_type.as_deref() {
            req = req.content_type(ct);
        }
        req.send().await.map_err(|e| map_err("put", e))?;
        Ok(UploadResult {
            url: join_url(&self.config.public_base_url, &input.path),
            path: input.path,
            size,
        })
    }

    async fn download(&self, path: &str) -> StorageResult<Vec<u8>> {
        let key = self.key(path);
        let output = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| map_err("get", e))?;
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| map_err("read", e))?;
        Ok(bytes.into_bytes().to_vec())
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        let key = self.key(path);
        match self
            .client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if is_missing(&e) => Ok(()),
            Err(e) => Err(map_err("delete", e)),
        }
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let key = self.key(path);
        match self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) if is_missing(&e) => Ok(false),
            Err(e) => Err(map_err("head", e)),
        }
    }

    async fn public_url(&self, path: &str) -> StorageResult<String> {
        Ok(join_url(&self.config.public_base_url, path))
    }

    async fn health_check(&self) -> StorageResult<()> {
        self.client
            .head_bucket()
            .bucket(&self.config.bucket)
            .send()
            .await
            .map_err(|e| map_err("head_bucket", e))?;
        Ok(())
    }
}
