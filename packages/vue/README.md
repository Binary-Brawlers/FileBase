# @binary-brawlers/filebase-vue

Vue 3 integration for [FileBase](https://github.com/binary-brawlers/filebase)
upload gateway.

Built on top of `@binary-brawlers/filebase-client`. Works with any Vue 3
app — Vite, Nuxt, etc.

## Install

```bash
npm install @binary-brawlers/filebase-vue
```

`vue >= 3` is a peer dependency.

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

Then point the composable at it. The Vue app calls that endpoint — never
the FileBase API directly.

## `useUpload` composable

```vue
<script setup lang="ts">
import { useUpload } from "@binary-brawlers/filebase-vue";

const {
  isUploading,
  progress,
  error,
  file,
  upload,
  abort,
  reset,
} = useUpload({
  signEndpoint: "/api/upload/sign",
  preset: "profile_images",
  onUploadComplete: (result) => console.log(result.url),
});

function onFileChange(event: Event) {
  const selected = (event.target as HTMLInputElement).files?.[0];
  if (selected) upload(selected);
}
</script>

<template>
  <div>
    <input type="file" :disabled="isUploading" @change="onFileChange" />
    <p v-if="isUploading">
      Uploading… {{ Math.round((progress?.fraction ?? 0) * 100) }}%
    </p>
    <p v-if="error" class="error">{{ error.code }}</p>
    <a v-if="file" :href="file.url">{{ file.url }}</a>
    <button type="button" :disabled="!isUploading" @click="abort">
      Cancel
    </button>
  </div>
</template>
```

`useUpload` returns a single reactive object (refs are auto-unwrapped in
templates):

```ts
{
  client: FileBaseClient;
  isUploading: Ref<boolean>;
  progress: Ref<{ loaded, total, fraction } | null>;
  error: Ref<FileBaseError | null>;
  file: Ref<FileBaseUploadResult | null>;
  upload: (file: Blob, overrides?: UploadOptions) => Promise<FileBaseUploadResult | null>;
  abort: () => void;
  reset: () => void;
}
```

Options mirror `FileBaseClientOptions` (`signEndpoint`, `signHeaders`,
`signCredentials`, `fetch`) plus:

- `preset` / `presetId` / `projectId` — preset to use server-side
- `onUploadComplete(file)`, `onUploadError(error)`

## Lower-level client

You can also use the underlying `FileBaseClient` directly, re-exported
from this package:

```ts
import { FileBaseClient, FileBaseError } from "@binary-brawlers/filebase-vue";

const client = new FileBaseClient({ signEndpoint: "/api/upload/sign" });
const result = await client.upload(file, { preset: "profile_images" });
```

## Errors

Catch with `error instanceof FileBaseError` to read `code` / `status` /
`details`. See [`@binary-brawlers/filebase-shared`](https://www.npmjs.com/package/@binary-brawlers/filebase-shared)
for the full code list.

## License

MIT
