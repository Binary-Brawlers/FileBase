# @binary-brawlers/filebase-client

Framework-agnostic browser SDK for uploading files to a [FileBase](https://github.com/binary-brawlers/filebase)
self-hosted upload gateway.

Use this package directly in any browser app (vanilla JS, Svelte, Solid,
Angular, etc.). If you're using React, React Native, Next.js, or Node.js,
prefer the dedicated package — they all use this client under the hood.

## Install

```bash
npm install @binary-brawlers/filebase-client
```

## How it works

The browser **never** sees your FileBase API key. Instead:

1. Your own backend exposes a **sign endpoint** that uses
   `@binary-brawlers/filebase-node` (or `@binary-brawlers/filebase-next`) to
   exchange the secret API key for a short-lived signed upload session.
2. The browser calls `FileBaseClient.upload(file)`, which:
   - POSTs to your sign endpoint to get a session,
   - then POSTs the file directly to the FileBase gateway with that session
     token.

## Quick start

```ts
import { FileBaseClient, FileBaseError } from "@binary-brawlers/filebase-client";

const client = new FileBaseClient({
  signEndpoint: "/api/upload/sign",   // your own backend route
});

async function handleFile(file: File) {
  try {
    const result = await client.upload(file, {
      preset: "profile_images",       // matches a preset in your FileBase dashboard
      onProgress: ({ loaded, total, fraction }) => {
        console.log(`uploading ${loaded}/${total}`, fraction);
      },
    });
    console.log("uploaded:", result.url);
  } catch (err) {
    if (err instanceof FileBaseError) console.error(err.code, err.details);
  }
}
```

## API

### `new FileBaseClient(options)`

```ts
type FileBaseClientOptions = {
  signEndpoint: string;                // required
  fetch?: typeof fetch;                // optional override
  signHeaders?: Record<string, string>;
  signCredentials?: RequestCredentials; // default "same-origin"
};
```

### `client.upload(file, options?)`

One-shot upload. Returns `Promise<FileBaseUploadResult>`.

```ts
type UploadOptions = {
  preset?: string;            // preset name
  presetId?: string;          // or preset id
  projectId?: string;
  expiresInSeconds?: number;  // override session TTL
  filename?: string;          // override file name in multipart
  contentType?: string;       // override mime
  fields?: Record<string, string>; // extra multipart fields
  onProgress?: (p: UploadProgress) => void;
  signal?: AbortSignal;
};
```

### `client.createSession(request?)`

Just gets a session without uploading. Returns
`Promise<FileBaseUploadSession>`.

### `client.uploadToSession(session, file, options?)`

Upload to a pre-fetched session (useful if your backend hands the session to
the browser as part of another response).

### Cancellation

```ts
const controller = new AbortController();
client.upload(file, { signal: controller.signal });
controller.abort();
```

## Sign endpoint contract

Your backend route must accept a `POST` with an optional JSON body of shape
`{ preset?, presetId?, projectId?, expiresInSeconds? }` and respond with:

```json
{ "data": { "id": "…", "uploadUrl": "…", "token": "…", "expiresAt": "…" } }
```

The easiest way to implement this is with
`@binary-brawlers/filebase-next` (App Router) or
`@binary-brawlers/filebase-node` (Express, Fastify, etc.).

## Errors

Throws `FileBaseError` (re-exported from
`@binary-brawlers/filebase-shared`). See its `code` field for branching.

## License

MIT
