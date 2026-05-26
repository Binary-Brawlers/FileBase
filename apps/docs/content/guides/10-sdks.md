# SDKs

FileBase ships first-party SDKs for the most common JavaScript runtimes. They wrap the [signed upload](./09-signed-uploads.md) flow so you don't have to call `/uploads/sign` and the upload endpoint by hand.

## Packages

| Package | Use case |
| --- | --- |
| `@binary-brawlers/filebase-client` | Browser core. Calls your sign endpoint and uploads files. |
| `@binary-brawlers/filebase-react` | React hooks and components (`useUpload`, `UploadButton`, `UploadDropzone`). |
| `@binary-brawlers/filebase-react-native` | React Native / Expo uploads from a file URI. |
| `@binary-brawlers/filebase-next` | Next.js route helpers for App and Pages routers. |
| `@binary-brawlers/filebase-node` | Server-side admin client. |
| `@binary-brawlers/filebase-vue` | (Planned) Vue components. |

Install with your package manager of choice:

```bash
pnpm add @binary-brawlers/filebase-react
```

## Next.js: sign endpoint

Expose a server route that issues signed sessions:

```ts
// app/api/upload/sign/route.ts
import { createFileBaseRoute } from "@binary-brawlers/filebase-next";

export const POST = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
});
```

`FILEBASE_API_KEY` is an `fb_live_` key. `FILEBASE_GATEWAY_URL` is the FileBase API base URL, e.g. `https://uploads.example.com`.

## React: drop-in component

```tsx
import { UploadButton } from "@binary-brawlers/filebase-react";

export function ProfileUploader() {
  return (
    <UploadButton
      signEndpoint="/api/upload/sign"
      preset="profile_images"
      accept="image/*"
      maxSize={5 * 1024 * 1024}
      onUploadComplete={(file) => console.log(file.url)}
      onError={(err) => console.error(err)}
    />
  );
}
```

`UploadDropzone` is the drag-and-drop variant with the same props. `useUpload` is the underlying hook if you want to render your own UI.

## Core client (vanilla browser)

```ts
import { FileBaseClient } from "@binary-brawlers/filebase-client";

const client = new FileBaseClient({ signEndpoint: "/api/upload/sign" });

const result = await client.upload(file, {
  preset: "profile_images",
  onProgress: ({ loaded, total }) => console.log(loaded / total),
});

console.log(result.url);
```

## React Native

```tsx
import { uploadFile } from "@binary-brawlers/filebase-react-native";

const result = await uploadFile({
  uri: image.uri,
  name: "profile.jpg",
  type: "image/jpeg",
  signEndpoint: "https://api.example.com/upload/sign",
  preset: "profile_images",
});
```

## Node admin client

For server-side workflows (cleanup jobs, migrations, admin scripts):

```ts
import { FileBase } from "@binary-brawlers/filebase-node";

const fb = new FileBase({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
});

const session = await fb.createUploadSession({ preset: "profile_images" });
await fb.deleteFile("file_123");
const files = await fb.listFiles({ projectId: "prj_abc" });
```

## Errors

All SDKs surface structured errors with a `code` field:

| Code | Meaning |
| --- | --- |
| `mime_not_allowed` | File type rejected by the preset. |
| `file_too_large` | File exceeds the preset or session size cap. |
| `token_expired` | Signed session expired before upload completed. |
| `token_invalid` | Token does not match the session. |
| `duplicate_rejected` | Preset uses `reject_duplicate` and the same hash already exists. |
| `network_error` | Browser/network-level failure. |

Use `try/catch` around uploads and surface human-readable messages from `error.message`.
