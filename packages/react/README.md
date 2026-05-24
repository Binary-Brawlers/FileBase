# @binary-brawlers/filebase-react

React hooks and components for uploading files to a [FileBase](https://github.com/binary-brawlers/filebase)
upload gateway.

Built on top of `@binary-brawlers/filebase-client`. Works with any React
18/19 app — Next.js, Vite, CRA, Remix, Astro islands, etc.

## Install

```bash
npm install @binary-brawlers/filebase-react
```

`react >= 18` is a peer dependency.

## Setup

You need a **sign endpoint** on your own backend that exchanges your secret
FileBase API key for a short-lived upload session. The fastest way:

```ts
// app/api/upload/sign/route.ts  (Next.js App Router)
import { createFileBaseRoute } from "@binary-brawlers/filebase-next";

export const POST = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY!,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL!,
});
```

Then point the React components at it.

## `useUpload` hook

```tsx
import { useUpload } from "@binary-brawlers/filebase-react";

function Avatar() {
  const upload = useUpload({
    signEndpoint: "/api/upload/sign",
    preset: "profile_images",
    onUploadComplete: (file) => console.log(file.url),
  });

  return (
    <input
      type="file"
      disabled={upload.isUploading}
      onChange={(e) => {
        const file = e.target.files?.[0];
        if (file) upload.upload(file);
      }}
    />
  );
}
```

Returned state:

```ts
{
  isUploading: boolean;
  progress: { loaded, total, fraction } | null;
  error: FileBaseError | null;
  file: FileBaseUploadResult | null;
  upload: (file: Blob, overrides?) => Promise<FileBaseUploadResult | null>;
  abort: () => void;
  reset: () => void;
  client: FileBaseClient;
}
```

## `<UploadButton>`

A styled-anywhere file picker that uploads on change.

```tsx
import { UploadButton } from "@binary-brawlers/filebase-react";

<UploadButton
  signEndpoint="/api/upload/sign"
  preset="profile_images"
  accept="image/*"
  maxSize={5 * 1024 * 1024}
  onUploadComplete={(file) => console.log(file.url)}
  className="rounded bg-black px-4 py-2 text-white"
>
  Pick a photo
</UploadButton>
```

Pass a render function to access live state:

```tsx
<UploadButton signEndpoint="/api/upload/sign" preset="profile_images">
  {({ isUploading, progress }) =>
    isUploading
      ? `Uploading… ${Math.round((progress?.fraction ?? 0) * 100)}%`
      : "Upload"
  }
</UploadButton>
```

## `<UploadDropzone>`

Drag-and-drop zone that also opens the picker on click.

```tsx
import { UploadDropzone } from "@binary-brawlers/filebase-react";

<UploadDropzone
  signEndpoint="/api/upload/sign"
  preset="documents"
  accept=".pdf,.docx"
  multiple
  maxSize={20 * 1024 * 1024}
  onUploadComplete={(file) => console.log(file.url)}
  className="rounded border-2 border-dashed p-8 text-center"
/>
```

## Shared props

All three APIs accept:

- `signEndpoint` (required) — your backend route
- `preset` / `presetId` / `projectId` — preset to use server-side
- `signHeaders`, `signCredentials`, `fetch` — passed to the underlying
  `FileBaseClient`
- `onUploadComplete(file)`, `onUploadError(error)`

Components additionally accept: `accept`, `multiple`, `maxSize`, `disabled`,
`className`, `style`, and `children`.

## Errors

Catch with `error instanceof FileBaseError` to read `code` / `status` /
`details`. See [`@binary-brawlers/filebase-shared`](https://www.npmjs.com/package/@binary-brawlers/filebase-shared)
for the full code list.

## License

MIT
