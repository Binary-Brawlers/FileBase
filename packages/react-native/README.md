# @binary-brawlers/filebase-react-native

React Native / Expo SDK for uploading files to a [FileBase](https://github.com/binary-brawlers/filebase)
upload gateway. Works with `expo-image-picker`, `expo-document-picker`,
`react-native-image-picker`, and any other source that gives you a local
`uri`.

## Install

```bash
npm install @binary-brawlers/filebase-react-native
# or
yarn add @binary-brawlers/filebase-react-native
# or
pnpm add @binary-brawlers/filebase-react-native
```

Peer dependencies: `react >= 18`, `react-native >= 0.73`. Works on Expo SDK
50+.

## Setup

You need a **sign endpoint** on your backend that exchanges your secret
FileBase API key for a short-lived upload session. The mobile app calls
that endpoint, never the FileBase API directly. See
[`@binary-brawlers/filebase-next`](https://www.npmjs.com/package/@binary-brawlers/filebase-next) or
[`@binary-brawlers/filebase-node`](https://www.npmjs.com/package/@binary-brawlers/filebase-node)
for ready-made server helpers.

## One-shot upload

```ts
import * as ImagePicker from "expo-image-picker";
import { uploadFile } from "@binary-brawlers/filebase-react-native";

async function pickAndUpload() {
  const picked = await ImagePicker.launchImageLibraryAsync({
    mediaTypes: ImagePicker.MediaTypeOptions.Images,
    quality: 0.9,
  });
  if (picked.canceled) return;
  const asset = picked.assets[0];

  const result = await uploadFile({
    uri: asset.uri,
    name: asset.fileName ?? "photo.jpg",
    type: asset.mimeType ?? "image/jpeg",
    signEndpoint: "https://api.example.com/upload/sign",
    preset: "profile_images",
    onProgress: ({ fraction }) => console.log("progress", fraction),
  });

  console.log("uploaded:", result.url);
}
```

## `useUpload` hook

```tsx
import { Button, View, Text } from "react-native";
import * as DocumentPicker from "expo-document-picker";
import { useUpload } from "@binary-brawlers/filebase-react-native";

export function UploadScreen() {
  const upload = useUpload({
    signEndpoint: "https://api.example.com/upload/sign",
    preset: "documents",
    onUploadComplete: (file) => console.log("done", file.url),
  });

  async function pick() {
    const res = await DocumentPicker.getDocumentAsync({ type: "*/*" });
    if (res.canceled) return;
    const f = res.assets[0];
    upload.upload({ uri: f.uri, name: f.name, type: f.mimeType, size: f.size });
  }

  return (
    <View>
      <Button title="Pick file" onPress={pick} disabled={upload.isUploading} />
      {upload.progress && (
        <Text>{Math.round((upload.progress.fraction ?? 0) * 100)}%</Text>
      )}
      {upload.error && <Text>Error: {upload.error.code}</Text>}
      {upload.file && <Text>Uploaded: {upload.file.url}</Text>}
    </View>
  );
}
```

## Lower-level client

```ts
import { FileBaseNativeClient } from "@binary-brawlers/filebase-react-native";

const client = new FileBaseNativeClient({
  signEndpoint: "https://api.example.com/upload/sign",
});

const session = await client.createSession({ preset: "profile_images" });
const file = await client.uploadToSession(session, {
  uri: "file:///path/to/local.jpg",
  name: "local.jpg",
  type: "image/jpeg",
});
```

## File shape

```ts
type FileBaseNativeFile = {
  uri: string;          // file:// or content:// uri from the picker
  name?: string;        // shown to FileBase; defaults to the filename in the uri
  type?: string;        // mime; defaults to application/octet-stream
  size?: number;        // optional, informational
};
```

The SDK uses React Native's `FormData` `{ uri, name, type }` extension and
`XMLHttpRequest` (so progress events work on iOS and Android).

## Cancellation

```ts
const controller = new AbortController();
upload.upload(file, { signal: controller.signal });
controller.abort();
```

`useUpload().abort()` also cancels the in-flight request.

## Errors

Throws `FileBaseError` with a `code` field — `sign_failed`, `upload_failed`,
`network_error`, `validation_error`, `aborted`, or `unknown`.

## License

MIT
