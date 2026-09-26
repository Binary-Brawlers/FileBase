# CDN Integration

FileBase can serve file URLs through a CDN or custom public host. Set `CDN_BASE_URL` and every URL FileBase writes for new uploads uses that base instead of the storage connection's `public_base_url`:

```env
CDN_BASE_URL=https://cdn.example.com
```

For example, a file stored at `media/logo.avif` is returned as `https://cdn.example.com/media/logo.avif` instead of `http://192.0.2.10:8080/uploads/media/logo.avif`.

## How it works

- The CDN base replaces the public base URL of every storage adapter (local, FTP, SFTP, S3-compatible) when FileBase uploads files.
- The replacement is applied at upload time, so URLs stored in the database, upload logs, webhook payloads, and API responses all point at the CDN.
- Existing files keep the URLs recorded at upload time. To move an existing installation to a CDN, update the affected rows, for example:

  ```sql
  UPDATE files
  SET url = replace(url, 'http://192.0.2.10:8080/uploads', 'https://cdn.example.com');
  ```

## CDN setup

Your CDN should pull from the origin that actually serves the files:

1. **Object storage (S3, R2, Spaces, B2, Wasabi):** point the CDN at the bucket or its public origin. This is the simplest setup because the CDN and FileBase both read the same objects.
2. **FTP/SFTP/local:** expose the storage location through a web server (for example an nginx `root` for local uploads or an HTTP gateway for the remote server) and point the CDN at that origin.

FileBase does not need to be the origin. Keep `PUBLIC_BASE_URL` pointing at the origin if you ever need to bypass the CDN.

## Dynamic transforms and CDNs

Dynamic transform URLs are served by the FileBase API (`/transform/:preset/:file_id`). To accelerate them:

- Add a CDN cache rule for `/transform/*` and forward `ETag` and `Cache-Control`. FileBase already sends `Cache-Control: public, max-age=<TRANSFORM_CACHE_TTL_SECONDS>`.
- Because transformations are deterministic per URL, the CDN can cache them safely. Query strings must be part of the cache key.
- Alternatively, generate variants at upload time with [advanced thumbnails](./21-dynamic-transforms.md) and let the CDN serve the stored files.

## Verifying

```bash
curl -s http://localhost:8080/files | jq '.data[0].url'
```

New uploads return CDN URLs immediately. The files dashboard, copy-URL action, and webhooks show the same URLs.
