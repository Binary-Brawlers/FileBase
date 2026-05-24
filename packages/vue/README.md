# @binary-brawlers/filebase-vue

Vue 3 integration for [FileBase](https://github.com/binary-brawlers/filebase).

> **Status:** Early preview. This package currently re-exports
> `FileBaseClient` from `@binary-brawlers/filebase-client` so Vue users can
> install a single package and start uploading today. Composition-API
> composables (`useFileBaseUpload`) and components (`<FileBaseUpload>`,
> `<FileBaseDropzone>`) are tracked on the FileBase roadmap and will land in
> a future minor release without breaking the current API.

## Install

```bash
npm install @binary-brawlers/filebase-vue
```

Peer dependency: `vue >= 3`.

## Setup

You need a **sign endpoint** on your backend (see
[`@binary-brawlers/filebase-node`](https://www.npmjs.com/package/@binary-brawlers/filebase-node)
or [`@binary-brawlers/filebase-next`](https://www.npmjs.com/package/@binary-brawlers/filebase-next))
that returns a signed upload session. The Vue app calls that endpoint —
never the FileBase API directly.

## Today: use `FileBaseClient`

```vue
<script setup lang="ts">
import { ref } from "vue";
import { FileBaseClient, FileBaseError } from "@binary-brawlers/filebase-vue";

const client = new FileBaseClient({ signEndpoint: "/api/upload/sign" });
const isUploading = ref(false);
const progress = ref(0);
const url = ref<string | null>(null);
const error = ref<string | null>(null);

async function onChange(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  isUploading.value = true;
  error.value = null;
  try {
    const result = await client.upload(file, {
      preset: "profile_images",
      onProgress: (p) => {
        progress.value = Math.round((p.fraction ?? 0) * 100);
      },
    });
    url.value = result.url;
  } catch (e) {
    error.value = e instanceof FileBaseError ? e.code : "upload failed";
  } finally {
    isUploading.value = false;
  }
}
</script>

<template>
  <div>
    <input type="file" :disabled="isUploading" @change="onChange" />
    <p v-if="isUploading">Uploading… {{ progress }}%</p>
    <p v-if="error" class="error">{{ error }}</p>
    <a v-if="url" :href="url">{{ url }}</a>
  </div>
</template>
```

## Wrap as a composable

A minimal `useUpload` is a few lines of Composition API on top of the
client:

```ts
// composables/useUpload.ts
import { reactive } from "vue";
import { FileBaseClient, FileBaseError } from "@binary-brawlers/filebase-vue";

export function useUpload(options: { signEndpoint: string; preset?: string }) {
  const client = new FileBaseClient({ signEndpoint: options.signEndpoint });
  const state = reactive({
    isUploading: false,
    progress: 0,
    error: null as string | null,
    url: null as string | null,
  });

  async function upload(file: File) {
    state.isUploading = true;
    state.error = null;
    try {
      const result = await client.upload(file, {
        preset: options.preset,
        onProgress: (p) => (state.progress = Math.round((p.fraction ?? 0) * 100)),
      });
      state.url = result.url;
      return result;
    } catch (e) {
      state.error = e instanceof FileBaseError ? e.code : "upload failed";
    } finally {
      state.isUploading = false;
    }
  }

  return { state, upload };
}
```

The first-party composables and components will follow the same shape so
you can swap them in later.

## License

MIT
