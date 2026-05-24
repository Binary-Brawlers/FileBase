# @binary-brawlers/filebase-node

Server-side admin SDK for [FileBase](https://github.com/binary-brawlers/filebase).
Use it from any Node.js (or Bun / Deno with `fetch`) backend to mint signed
upload sessions, upload files directly from your server, and manage files.

If you're on Next.js, prefer [`@binary-brawlers/filebase-next`](https://www.npmjs.com/package/@binary-brawlers/filebase-next)
which wraps this package with a route helper.

## Install

```bash
npm install @binary-brawlers/filebase-node
```

Requires Node 18+ (for the global `fetch` and `FormData`).

## Configure

```ts
import { FileBase } from "@binary-brawlers/filebase-node";

const filebase = new FileBase({
  apiKey: process.env.FILEBASE_API_KEY!,          // fb_live_… or fb_test_…
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,  // e.g. https://uploads.example.com
});
```

> Never expose `apiKey` to the browser.

## Mint a signed upload session

This is the typical flow: your frontend asks your backend for a session, and
the browser uploads directly to the FileBase gateway.

```ts
// e.g. in an Express handler
app.post("/upload/sign", async (req, res) => {
  const session = await filebase.createUploadSession({
    preset: "profile_images",
    projectId: req.user.projectId,
  });
  res.json({ data: session });
});
```

The shape `{ data: session }` is exactly what the browser SDKs
(`@binary-brawlers/filebase-client`, `-react`, `-react-native`) expect from
their `signEndpoint`.

## Server-side upload

Upload bytes you already have on the server (a generated PDF, a webhook
payload, an S3 import, etc.):

```ts
import { readFile } from "node:fs/promises";

const buffer = await readFile("./report.pdf");
const file = await filebase.uploadFile(buffer, {
  preset: "exports",
  filename: "monthly-report.pdf",
  contentType: "application/pdf",
});

console.log(file.url, file.size, file.hash);
```

Accepts `Blob`, `ArrayBuffer`, or `Uint8Array`.

## Manage files

```ts
const files = await filebase.listFiles({
  projectId: "prj_…",
  search: "invoice",
  mimeType: "application/pdf",
  from: "2025-01-01",
  to: "2025-12-31",
});

const one = await filebase.getFile("file_…");
await filebase.deleteFile("file_…");
```

## API reference

### `new FileBase(options)`

```ts
type FileBaseOptions = {
  apiKey: string;
  gatewayUrl: string;
  fetch?: typeof fetch;   // override for tests / proxies
};
```

### Methods

- `createUploadSession(opts?) → Promise<FileBaseUploadSession>`
- `uploadFile(body, opts?) → Promise<FileBaseFile>`
- `listFiles(query?) → Promise<FileBaseFile[]>`
- `getFile(id) → Promise<FileBaseFile>`
- `deleteFile(id) → Promise<void>`

All accept either camelCase (`projectId`, `presetId`) or string-typed
options; the SDK snake-cases them for the wire.

## Errors

Every method throws `FileBaseError` on failure with `code`, `status`, and
`details`. See [`@binary-brawlers/filebase-shared`](https://www.npmjs.com/package/@binary-brawlers/filebase-shared)
for the code list.

```ts
import { FileBaseError } from "@binary-brawlers/filebase-node";

try {
  await filebase.uploadFile(buf, { preset: "exports" });
} catch (e) {
  if (e instanceof FileBaseError) {
    console.error(e.code, e.status, e.details);
  }
}
```

## License

MIT
