# Upload Presets

A preset is a reusable upload rule. Instead of validating files and configuring transformations in your application code, you define a preset once in FileBase and reference it by name when uploading.

## Why presets

- **Validation in one place.** MIME types, extensions, and size limits are enforced server-side.
- **Consistent file layout.** Every upload to `profile_images` lands in the same folder.
- **Reusable transformations.** Compression, resizing, and format conversion are configured per preset.
- **Frontend safety.** A signed upload session inherits the preset's rules, so a browser cannot bypass them.

## Fields

| Field | Description |
| --- | --- |
| `name` | URL-safe identifier, e.g. `profile_images`. Unique per project. |
| `folder` | Destination folder relative to the storage connection's `basePath`, e.g. `users/profiles`. |
| `allowedMimeTypes` | Array of MIME types, e.g. `["image/jpeg", "image/png", "image/webp"]`. |
| `allowedExtensions` | Optional array of extensions, e.g. `["jpg", "png", "webp"]`. |
| `maxFileSize` | Maximum size in bytes. |
| `duplicateStrategy` | `return_existing` (default), `upload_new_copy`, or `reject_duplicate`. See [Quick Start](./01-quick-start.md). |
| `filenameStrategy` | `slug-random` (default), `uuid`, `hash`, `timestamp-random`, `random`, or `original-suffix`. |
| `transformations` | Optional object describing image processing. |
| `storageConnectionId` | Which storage connection to upload to. |

## Example

```json
{
  "name": "profile_images",
  "folder": "users/profiles",
  "allowedMimeTypes": ["image/jpeg", "image/png", "image/webp"],
  "maxFileSize": 5242880,
  "duplicateStrategy": "return_existing",
  "filenameStrategy": "slug-random",
  "transformations": {
    "format": "webp",
    "quality": 80,
    "resize": { "width": 500, "height": 500, "fit": "cover" },
    "thumbnail": { "width": 100, "height": 100 }
  }
}
```

Create from the dashboard or:

```bash
curl -X POST http://localhost:8080/upload-presets \
  -H "Authorization: Bearer fb_live_..." \
  -H "Content-Type: application/json" \
  -d @preset.json
```

## Transformation options

| Key | Description |
| --- | --- |
| `format` | `webp`, `jpeg`, `png`, or omitted (preserve). |
| `quality` | Integer 1–100. Applies to lossy formats. |
| `resize.width` / `resize.height` | Target dimensions. |
| `resize.fit` | `cover`, `contain`, `inside`, or `outside`. |
| `thumbnail.width` / `thumbnail.height` | Generates an additional thumbnail. |
| `preserveOriginal` | If `true`, the original file is also kept. |

Transformations run in the background worker. The HTTP upload response returns immediately with the original file's URL; transformed assets are linked from the file's metadata once processing completes.

## Filename strategies

| Strategy | Example output |
| --- | --- |
| `slug-random` (default) | `my-photo-a82f91.webp` |
| `uuid` | `b3c8c0a8-2c0e-4d0d-9d5b-7e3f1c4a9b21.png` |
| `hash` | `9f86d081884c7d659a2feaa0c55ad015.png` |
| `timestamp-random` | `1716720000-9c2a.png` |
| `random` | `9c2a4d6f.png` |
| `original-suffix` | `My Photo-9c2a.png` |

Pick a strategy that matches how you expose URLs publicly. `slug-random` balances readability and uniqueness.
