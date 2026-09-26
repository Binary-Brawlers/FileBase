# Media Processing

FileBase applies preset transformations while it stores uploads. Image presets support compression, resizing, thumbnails, AVIF conversion, and watermarking. Video presets support metadata extraction and thumbnail generation.

## Image transformations

Image transformations live in the `transformations` object of an upload preset:

```json
{
  "enabled": true,
  "format": "webp",
  "quality": 80,
  "resize": { "width": 1600, "height": 1600, "mode": "fit" },
  "thumbnail": {
    "enabled": true,
    "width": 320,
    "height": 320,
    "format": "webp",
    "quality": 78
  },
  "watermark": {
    "enabled": true,
    "type": "text",
    "text": "example.com",
    "position": "bottom_right",
    "opacity": 0.6,
    "margin": 16,
    "scale": 2.0,
    "color": "#ffffff"
  },
  "preserve_original": true
}
```

| Field               | Values                                    | Notes                                                     |
| ------------------- | ----------------------------------------- | --------------------------------------------------------- |
| `enabled`           | boolean                                   | Master switch for image processing.                       |
| `format`            | `original`, `jpeg`, `png`, `webp`, `avif` | `original` keeps the source format when possible.         |
| `quality`           | 1–100                                     | Applied to JPEG and AVIF output.                          |
| `resize`            | object                                    | `width`, `height`, and `mode` (`fit` or `fill`).          |
| `thumbnail`         | object                                    | Square crop-to-fill thumbnail stored under `thumbnails/`. |
| `watermark`         | object                                    | See below.                                                |
| `preserve_original` | boolean                                   | Stores the untouched source under `originals/`.           |

AVIF encoding uses a speed setting of 4 and honors `quality`. AVIF uploads are stored unchanged because decoding AVIF input requires a native decoder that is not part of the default image feature set.

## Watermarking

Watermarks support text and image overlays. Both are blended onto the processed output before encoding.

```json
{
  "watermark": {
    "enabled": true,
    "type": "image",
    "image": "data:image/png;base64,iVBORw0KGgo...",
    "position": "center",
    "opacity": 0.35,
    "margin": 24,
    "width_percent": 20,
    "apply_to_thumbnail": true
  }
}
```

| Field                | Values                                                                                                                         | Default  |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------- |
| `type`               | `text`, `image`                                                                                                                | `text`   |
| `text`               | non-empty string                                                                                                               | —        |
| `image`              | base64 data URI or raw base64 of a PNG/JPEG/WebP/GIF                                                                           | —        |
| `position`           | `top_left`, `top_center`, `top_right`, `center_left`, `center`, `center_right`, `bottom_left`, `bottom_center`, `bottom_right` | `center` |
| `opacity`            | 0–1                                                                                                                            | `0.7`    |
| `margin`             | pixels                                                                                                                         | `16`     |
| `scale`              | text glyph size multiplier (0.1–32)                                                                                            | `2.0`    |
| `width_percent`      | image watermark width as a percentage of the output (1–100)                                                                    | `20`     |
| `color`              | 6-digit hex text color                                                                                                         | `ffffff` |
| `apply_to_thumbnail` | boolean                                                                                                                        | `false`  |

Watermark details are saved in the file's transformation metadata as `image.watermark`.

## Video metadata and thumbnails

Video processing runs through `ffprobe` and `ffmpeg`. The official API image includes ffmpeg. For manual installations, install ffmpeg on the host or point FileBase at custom binaries:

```env
FFPROBE_PATH=ffprobe
FFMPEG_PATH=ffmpeg
```

When a video is uploaded, FileBase probes it and stores duration, resolution, codec, frame rate, bit rate, and container in the file metadata under `video`. Probing is best effort: if ffprobe is unavailable the upload still succeeds.

To generate a video thumbnail, add a `video` section to the preset:

```json
{
  "enabled": false,
  "video": {
    "enabled": true,
    "thumbnail": {
      "enabled": true,
      "at_seconds": 1.5,
      "width": 640,
      "format": "jpeg",
      "quality": 82
    }
  }
}
```

| Field        | Values                    | Default |
| ------------ | ------------------------- | ------- |
| `at_seconds` | frame position in seconds | `1.0`   |
| `width`      | 1–4096 pixels             | `640`   |
| `format`     | `jpeg`, `png`, `webp`     | `jpeg`  |
| `quality`    | 1–100                     | `82`    |

Video thumbnails are stored under `thumbnails/` next to the uploaded file and are reported in `metadata.thumbnail`. If ffmpeg is missing or fails, FileBase logs a warning and keeps the original upload.

## Operational notes

- Processing runs synchronously during upload, so response time scales with file size.
- `max_file_size` is enforced again after processing; very large outputs are rejected.
- Check ffmpeg availability under **Dashboard → Operations → Media tooling**.
