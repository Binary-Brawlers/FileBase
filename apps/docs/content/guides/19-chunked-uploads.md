# Resumable Chunked Uploads

Chunked uploads split a file into parts that are uploaded independently and assembled on the server. This enables resumable transfers for large videos and unreliable networks: if a chunk fails, the client can query which chunks the server already holds and continue where it stopped.

Chunked uploads reuse signed upload sessions. Create a session with `POST /uploads/sign` exactly as described in [Signed Uploads](./09-signed-uploads.md), then use the session token (`Authorization: Bearer <session-token>`) on every chunk request.

## Flow

1. Create an upload session and keep `session.id`, `session.uploadUrl`, and `session.token`.
2. Optionally call `GET /uploads/:session_id/chunks` to discover chunks already stored for this session.
3. Upload each missing chunk with `POST /uploads/:session_id/chunks`.
4. Call `POST /uploads/:session_id/complete` with the original filename and content type.
5. The server assembles the chunks, validates the file, runs preset transformations, stores it, and marks the session as used.

## Endpoints

| Endpoint                             | Description                                                                 |
| ------------------------------------ | --------------------------------------------------------------------------- |
| `GET /uploads/:session_id/chunks`    | Returns `received_chunks`, `received_bytes`, `chunk_size`, limits, expiry.  |
| `POST /uploads/:session_id/chunks`   | Multipart form with a `chunk` file field and a `chunk_index` text field.    |
| `DELETE /uploads/:session_id/chunks` | Deletes stored chunks so the upload can start over.                         |
| `POST /uploads/:session_id/complete` | JSON body `{ "filename": "...", "content_type": "..." }`. Returns the file. |

Chunk indexes are zero-based and must be contiguous when completing the upload. Re-uploading an index replaces the previous chunk, so clients can retry safely. Each request is validated against the session's expiry, project, preset, and `max_file_size`.

## Example

```bash
SESSION=$(curl -s -X POST http://localhost:8080/uploads/sign \
  -H "Authorization: Bearer fb_live_xxx" \
  -H "Content-Type: application/json" \
  -d '{"preset":"videos"}')

SESSION_ID=$(echo "$SESSION" | jq -r '.data.id')
TOKEN=$(echo "$SESSION" | jq -r '.data.token')
BASE="http://localhost:8080/uploads/$SESSION_ID"

# Discover existing chunks (empty on a new session)
curl -s "$BASE/chunks" -H "Authorization: Bearer $TOKEN"

# Upload chunk 0 (5 MiB slices work well)
split -b 5242880 -d video.mp4 part-
for index in $(seq 0 $(ls part-* | wc -l | awk '{print $1-1}')); do
  curl -s -X POST "$BASE/chunks" \
    -H "Authorization: Bearer $TOKEN" \
    -F "chunk_index=$index" \
    -F "chunk=@part-0$index"
done

# Assemble and process
curl -s -X POST "$BASE/complete" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"filename":"video.mp4","content_type":"video/mp4"}'
```

## SDK

The JavaScript client uploads in chunks and resumes automatically:

```ts
import { FileBaseClient } from "@binary-brawlers/filebase-client";

const client = new FileBaseClient({ signEndpoint: "/api/filebase/sign" });

const file = await client.upload(fileInput.files[0], {
  preset: "videos",
  strategy: "chunked",
  chunkSize: 5 * 1024 * 1024,
  onProgress: (progress) => console.log(progress.fraction),
});
```

Low-level helpers are available when you already have a session:

- `client.uploadChunked(file, options)` — signs a new session and uploads in chunks.
- `client.uploadChunkedToSession(session, file, options)` — chunked upload to an existing session.
- `client.chunkStatus(session)` — returns the chunks the server already has.

## Server configuration

| Variable            | Default   | Description                        |
| ------------------- | --------- | ---------------------------------- |
| `UPLOAD_CHUNK_SIZE` | `5242880` | Chunk size reported to clients.    |
| `MAX_UPLOAD_SIZE`   | 10 MiB    | Maximum size of an assembled file. |

Each individual chunk request is also limited by `MAX_UPLOAD_SIZE`. Set chunk sizes below the server limit so multipart overhead does not push a request over the edge.

Stale chunks and expired sessions can be removed from **Dashboard → Operations** or with `POST /admin/maintenance/cleanup`.
