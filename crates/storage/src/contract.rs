//! Behaviour every [`StorageAdapter`] must share, so local, FTP, SFTP and S3
//! backends stay interchangeable. Adapter crates run [`run_adapter_contract`]
//! from their tests (enable the `contract-tests` feature as a dev-dependency).

use crate::{StorageAdapter, UploadInput};

/// Exercises the full upload → exists → download → overwrite → delete cycle
/// under `prefix`, panicking with the failing step on any mismatch.
///
/// `prefix` should be unique per run so parallel runs against a shared server
/// do not collide. `public_base_url` is the base the adapter was built with.
pub async fn run_adapter_contract(
    adapter: &dyn StorageAdapter,
    prefix: &str,
    public_base_url: &str,
) {
    if let Err(error) = adapter.health_check().await {
        panic!("health_check failed: {error}");
    }

    let path = format!("{prefix}/nested/dir/contract.bin");
    // NUL, 0xff and CR/LF catch backends that transcode bytes (e.g. FTP in
    // ASCII mode rewriting line endings).
    let original: &[u8] = b"first\r\nversion\n\x00\xff\x89PNG\r\n\x1a\n";
    let replacement: &[u8] = b"2nd";
    let uploaded = match adapter
        .upload(UploadInput {
            path: path.clone(),
            bytes: original.to_vec(),
            content_type: Some("application/octet-stream".into()),
        })
        .await
    {
        Ok(result) => result,
        Err(error) => panic!("upload failed: {error}"),
    };
    assert_eq!(uploaded.path, path);
    assert_eq!(uploaded.size, original.len() as u64);
    assert_eq!(uploaded.url, crate::join_url(public_base_url, &path));

    match adapter.public_url(&path).await {
        Ok(url) => assert_eq!(url, uploaded.url),
        Err(error) => panic!("public_url failed: {error}"),
    }
    match adapter.exists(&path).await {
        Ok(exists) => assert!(exists, "uploaded file should exist"),
        Err(error) => panic!("exists failed: {error}"),
    }
    match adapter.download(&path).await {
        Ok(bytes) => assert_eq!(bytes, original, "bytes must round-trip unchanged"),
        Err(error) => panic!("download failed: {error}"),
    }

    if let Err(error) = adapter
        .upload(UploadInput {
            path: path.clone(),
            bytes: replacement.to_vec(),
            content_type: None,
        })
        .await
    {
        panic!("overwrite upload failed: {error}");
    }
    match adapter.download(&path).await {
        Ok(bytes) => assert_eq!(
            bytes, replacement,
            "a shorter overwrite should fully replace the content"
        ),
        Err(error) => panic!("download after overwrite failed: {error}"),
    }

    if let Err(error) = adapter.delete(&path).await {
        panic!("delete failed: {error}");
    }
    match adapter.exists(&path).await {
        Ok(exists) => assert!(!exists, "deleted file should not exist"),
        Err(error) => panic!("exists after delete failed: {error}"),
    }
    if let Err(error) = adapter.delete(&path).await {
        panic!("deleting a missing file should succeed: {error}");
    }
    match adapter
        .exists(&format!("{prefix}/never-uploaded.txt"))
        .await
    {
        Ok(exists) => assert!(!exists),
        Err(error) => panic!("exists for missing file failed: {error}"),
    }
}

/// A per-run path prefix (`contract-<pid>-<unix nanos>`) so concurrent runs
/// from different processes or hosts against one server never share files.
pub fn unique_prefix() -> String {
    let nanos = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_nanos(),
        Err(_) => 0,
    };
    format!("contract-{}-{nanos}", std::process::id())
}
