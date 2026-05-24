# @binary-brawlers/filebase-shared

Shared TypeScript types and the `FileBaseError` class used by every other
FileBase SDK package. You usually don't install this directly — it comes as a
transitive dependency of `@binary-brawlers/filebase-client`,
`@binary-brawlers/filebase-react`, `@binary-brawlers/filebase-react-native`,
`@binary-brawlers/filebase-next`, and `@binary-brawlers/filebase-node`.

Install it explicitly only if you're building your own integration on top of
the FileBase HTTP API and want the type definitions.

## Install

```bash
npm install @binary-brawlers/filebase-shared
# or
pnpm add @binary-brawlers/filebase-shared
# or
yarn add @binary-brawlers/filebase-shared
```

## What it exports

```ts
import {
  FileBaseError,
  type FileBaseFile,
  type FileBaseUploadResult,
  type FileBaseUploadSession,
  type FileBaseSignRequest,
  type FileBaseErrorCode,
  type FileBaseEventName,
} from "@binary-brawlers/filebase-shared";
```

### `FileBaseError`

Every SDK throws this. It carries a machine-readable `code`, an optional HTTP
`status`, and a `details` payload from the server.

```ts
try {
  await client.upload(file);
} catch (err) {
  if (err instanceof FileBaseError) {
    console.error(err.code, err.status, err.details);
  }
}
```

| `code` | When it's thrown |
| --- | --- |
| `sign_failed` | Your sign endpoint returned a non-2xx or invalid payload. |
| `upload_failed` | The FileBase API rejected the upload (size, mime, etc.). |
| `network_error` | The request never reached the server. |
| `validation_error` | An option you passed in is missing or wrong. |
| `aborted` | The upload was cancelled via `AbortSignal`. |
| `unknown` | Catch-all. |

### Types

- `FileBaseFile` / `FileBaseUploadResult` — server file record returned after
  upload (`id`, `url`, `mimeType`, `size`, `hash`, `duplicate`, …).
- `FileBaseUploadSession` — `{ id, uploadUrl, token, expiresAt, … }` returned
  by your sign endpoint.
- `FileBaseSignRequest` — `{ preset?, presetId?, projectId?, expiresInSeconds? }`
  sent to your sign endpoint by the client SDKs.
- `FileBaseEventName` — `"file.uploaded" | "file.deleted" |
  "file.duplicate_detected" | "file.optimized" | "file.failed"`.

## License

MIT
