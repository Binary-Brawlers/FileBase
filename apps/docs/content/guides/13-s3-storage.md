# S3 & S3-Compatible Storage

S3 storage uploads files to an Amazon S3 bucket or any S3-compatible object store (Cloudflare R2, DigitalOcean Spaces, Backblaze B2, Wasabi, MinIO, and more). It is a good choice when you want managed, scalable object storage instead of a single server folder.

## When to use it

- You already use S3 or an S3-compatible object store.
- You want durable, off-box storage with CDN-friendly public URLs.
- You need to move beyond a single VPS filesystem, FTP, or SFTP.

## Configuration fields

Create an S3 connection from the dashboard or `POST /storage-connections`:

| Field | Description |
| --- | --- |
| `type` | `s3` |
| `bucket` | Bucket name, e.g. `uploads`. Must exist and be writable. |
| `region` | Region, e.g. `us-east-1`. Some providers accept a placeholder. |
| `endpoint` | Optional custom endpoint URL for S3-compatible providers, e.g. `https://<account>.r2.cloudflarestorage.com`. Leave blank for AWS, which uses the `region` endpoint automatically. |
| `accessKey` | Access key ID. Stored as `username` and never returned after creation. |
| `secretKey` | Secret access key. Encrypted with AES-256-GCM using `ENCRYPTION_KEY`. |
| `forcePathStyle` | Enable path-style addressing (`https://endpoint/bucket/key`) instead of virtual-hosted (`https://bucket.endpoint/key`). Required by several non-AWS providers and self-hosted MinIO. |
| `basePath` | Optional object key prefix, e.g. `public/uploads`. |
| `publicBaseUrl` | Public URL prefix, e.g. `https://cdn.example.com`. |

`accessKey`/`secretKey` are stored encrypted and are never returned by the API after creation.

## Provider examples

Amazon S3 (uses the region endpoint, no custom endpoint needed):

```json
{
  "type": "s3",
  "bucket": "my-bucket",
  "region": "us-east-1",
  "accessKey": "AKIA...",
  "secretKey": "...",
  "publicBaseUrl": "https://my-bucket.s3.amazonaws.com"
}
```

Cloudflare R2 (custom endpoint, path style):

```json
{
  "type": "s3",
  "bucket": "my-bucket",
  "region": "auto",
  "endpoint": "https://<account>.r2.cloudflarestorage.com",
  "accessKey": "<R2 access key>",
  "secretKey": "<R2 secret key>",
  "forcePathStyle": true,
  "publicBaseUrl": "https://pub-<hash>.r2.dev"
}
```

Wasabi, DigitalOcean Spaces, and Backblaze B2 use the same shape: set `endpoint` to the provider's S3 endpoint, pick the supplied `region`, and enable `forcePathStyle` where the provider requires it.

## Permissions

The access key needs `PutObject`, `GetObject`, `DeleteObject`, `ListBucket`, and `HeadBucket` on the bucket (and on the `basePath` prefix). A minimal policy scoped to the bucket is recommended so the key cannot affect other buckets.

## Test the connection

`POST /storage-connections/:id/test` performs an `HeadBucket` request. Common failure causes:

| Error | Likely cause |
| --- | --- |
| `403 Forbidden` | The access key lacks `HeadBucket`/`ListBucket` permission. |
| `NoSuchBucket` | Wrong bucket name or region. |
| `connection refused` | Wrong or unreachable `endpoint`. |
| `SignatureDoesNotMatch` | Wrong region or incorrectly signed request; check `region` and `forcePathStyle`. |
