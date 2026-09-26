# Dynamic Image Transformations

FileBase can transform images on demand from a URL instead of only at upload time. This is useful when the exact crop or size is only known at render time, for example responsive `srcset` images or one-off profile crops.

Dynamic transforms are opt-in per upload preset:

```json
{
  "enabled": true,
  "format": "webp",
  "url_transforms": { "enabled": true },
  "watermark": {
    "enabled": true,
    "type": "text",
    "text": "example.com",
    "position": "bottom_right"
  }
}
```

`format`, `quality`, and `resize` from the preset act as defaults; the watermark always applies. `thumbnail`, `thumbnails`, and `preserve_original` are ignored for dynamic requests.

## Endpoint

```http
GET /transform/:preset/:file_id?w=&h=&fit=&format=&q=
```

`preset` is the preset id or its unique name. `file_id` comes from any FileBase upload response or the files dashboard.

| Parameter | Values                                    | Notes                                               |
| --------- | ----------------------------------------- | --------------------------------------------------- |
| `w`       | 1–4096                                    | Target width.                                       |
| `h`       | 1–4096                                    | Target height.                                      |
| `fit`     | `fit`, `fill`                             | `fit` preserves aspect ratio, `fill` crops to size. |
| `format`  | `original`, `jpeg`, `png`, `webp`, `avif` | Output format override.                             |
| `q`       | 1–100                                     | Output quality override.                            |

Example:

```html
<img
  src="https://uploads.example.com/transform/profile_images/file_123?w=96&h=96&fit=fill&format=webp&q=75"
  srcset="
    /transform/profile_images/file_123?w=96&h=96&fit=fill&format=webp   1x,
    /transform/profile_images/file_123?w=192&h=192&fit=fill&format=webp 2x
  "
/>
```

## Caching and limits

- Transformed variants are cached on disk under the server temp directory (`filebase-transform-cache`).
- Responses include `ETag`, `Cache-Control: public, max-age=<ttl>`, and `X-FileBase-Transform-Cache: hit|miss`. Clients that send `If-None-Match` receive `304 Not Modified`.
- `TRANSFORM_CACHE_TTL_SECONDS` (default `86400`) controls how long a cached variant is reused.
- `TRANSFORM_MAX_DIMENSION` (default `4096`) caps requested width and height.
- `TRANSFORM_RATE_LIMIT_PER_MINUTE` (default `240`) limits transform requests per client IP.
- The source file must be smaller than `MAX_UPLOAD_SIZE`. AVIF sources are served unchanged because decoding AVIF input requires a native decoder that is not part of the default feature set.
- Stale entries can be removed with **Dashboard → Operations → Transform cache** or `POST /admin/maintenance/cleanup` with `{"scope":"transform_cache"}`.

Dynamic transforms work with every storage backend because FileBase fetches the source through the storage adapter. For CDN setups, see [CDN Integration](./22-cdn.md).

## Advanced thumbnails

Upload presets can also generate several thumbnail sizes up front with the `thumbnails` array (up to five entries, in addition to `thumbnail`):

```json
{
  "enabled": true,
  "thumbnail": { "enabled": true, "width": 320, "height": 320 },
  "thumbnails": [
    { "width": 640, "height": 640, "format": "jpeg", "quality": 80 },
    { "width": 160, "height": 160, "format": "webp", "quality": 75 }
  ]
}
```

Extra thumbnails are stored under `thumbnails/` as `<name>-thumb-<width>x<height>.<ext>` and are listed in the file metadata under `thumbnails` (the first entry mirrors `thumbnail`). The files dashboard shows the generated sizes in the file detail panel.
