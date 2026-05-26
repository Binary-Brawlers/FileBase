import { ref, shallowRef, type Ref } from "vue";
import {
  FileBaseClient,
  FileBaseError,
  type FileBaseClientOptions,
  type FileBaseUploadResult,
  type UploadOptions,
  type UploadProgress,
} from "@binary-brawlers/filebase-client";

export type UseUploadOptions = FileBaseClientOptions & {
  preset?: string;
  presetId?: string;
  projectId?: string;
  onUploadComplete?: (file: FileBaseUploadResult) => void;
  onUploadError?: (error: FileBaseError) => void;
};

export type UseUploadReturn = {
  client: FileBaseClient;
  isUploading: Ref<boolean>;
  progress: Ref<UploadProgress | null>;
  error: Ref<FileBaseError | null>;
  file: Ref<FileBaseUploadResult | null>;
  upload: (file: Blob, overrides?: UploadOptions) => Promise<FileBaseUploadResult | null>;
  abort: () => void;
  reset: () => void;
};

export function useUpload(options: UseUploadOptions): UseUploadReturn {
  const { preset, presetId, projectId, onUploadComplete, onUploadError, ...clientOptions } = options;
  const client = new FileBaseClient(clientOptions);

  const isUploading = ref(false);
  const progress = shallowRef<UploadProgress | null>(null);
  const error = shallowRef<FileBaseError | null>(null);
  const file = shallowRef<FileBaseUploadResult | null>(null);

  let controller: AbortController | null = null;

  const reset = () => {
    controller?.abort();
    controller = null;
    isUploading.value = false;
    progress.value = null;
    error.value = null;
    file.value = null;
  };

  const abort = () => {
    controller?.abort();
    controller = null;
  };

  const upload = async (
    blob: Blob,
    overrides: UploadOptions = {}
  ): Promise<FileBaseUploadResult | null> => {
    controller?.abort();
    const current = new AbortController();
    controller = current;
    isUploading.value = true;
    progress.value = null;
    error.value = null;
    file.value = null;
    try {
      const result = await client.upload(blob, {
        preset,
        presetId,
        projectId,
        ...overrides,
        signal: overrides.signal ?? current.signal,
        onProgress: (p) => {
          progress.value = p;
          overrides.onProgress?.(p);
        },
      });
      isUploading.value = false;
      progress.value = null;
      file.value = result;
      onUploadComplete?.(result);
      return result;
    } catch (cause) {
      const err =
        cause instanceof FileBaseError
          ? cause
          : new FileBaseError("unknown", "upload failed", { cause });
      isUploading.value = false;
      progress.value = null;
      error.value = err;
      onUploadError?.(err);
      return null;
    } finally {
      if (controller === current) {
        controller = null;
      }
    }
  };

  return { client, isUploading, progress, error, file, upload, abort, reset };
}
