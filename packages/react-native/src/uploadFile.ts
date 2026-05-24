import { FileBaseNativeClient } from "./client";
import type { UploadFileInput, UploadFileResult } from "./types";

/** One-shot upload helper. Creates a session and uploads the file. */
export async function uploadFile(input: UploadFileInput): Promise<UploadFileResult> {
  const {
    uri,
    name,
    type,
    size,
    signEndpoint,
    signHeaders,
    preset,
    presetId,
    projectId,
    expiresInSeconds,
    filename,
    contentType,
    fields,
    onProgress,
    signal,
  } = input;
  const client = new FileBaseNativeClient({ signEndpoint, signHeaders });
  return client.upload(
    { uri, name, type, size },
    {
      preset,
      presetId,
      projectId,
      expiresInSeconds,
      filename,
      contentType,
      fields,
      onProgress,
      signal,
    }
  );
}
