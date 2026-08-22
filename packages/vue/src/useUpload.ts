import {
  getCurrentScope,
  onScopeDispose,
  ref,
  shallowRef,
  type Ref,
} from "vue";
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
  upload: (
    file: Blob,
    overrides?: UploadOptions,
  ) => Promise<FileBaseUploadResult | null>;
  abort: () => void;
  reset: () => void;
};

export function useUpload(options: UseUploadOptions): UseUploadReturn {
  const {
    preset,
    presetId,
    projectId,
    onUploadComplete,
    onUploadError,
    ...clientOptions
  } = options;
  const client = new FileBaseClient(clientOptions);

  const isUploading = ref(false);
  const progress = shallowRef<UploadProgress | null>(null);
  const error = shallowRef<FileBaseError | null>(null);
  const file = shallowRef<FileBaseUploadResult | null>(null);

  let controller: AbortController | null = null;

  const abort = () => {
    const current = controller;
    controller = null;
    current?.abort();
    isUploading.value = false;
    progress.value = null;
  };

  const reset = () => {
    abort();
    error.value = null;
    file.value = null;
  };

  const upload = async (
    blob: Blob,
    overrides: UploadOptions = {},
  ): Promise<FileBaseUploadResult | null> => {
    controller?.abort();
    const current = new AbortController();
    controller = current;

    const externalSignal = overrides.signal;
    const abortFromExternalSignal = () => current.abort(externalSignal?.reason);
    if (externalSignal?.aborted) {
      abortFromExternalSignal();
    } else {
      externalSignal?.addEventListener("abort", abortFromExternalSignal, {
        once: true,
      });
    }

    isUploading.value = true;
    progress.value = null;
    error.value = null;
    file.value = null;

    let rejectOnAbort: (() => void) | null = null;
    try {
      let result: FileBaseUploadResult;
      try {
        if (current.signal.aborted) {
          throw new FileBaseError("aborted", "upload was aborted");
        }

        // The client may still be creating a session, so make cancellation settle immediately.
        const aborted = new Promise<never>((_, reject) => {
          rejectOnAbort = () =>
            reject(new FileBaseError("aborted", "upload was aborted"));
          current.signal.addEventListener("abort", rejectOnAbort, {
            once: true,
          });
        });

        result = await Promise.race([
          client.upload(blob, {
            preset,
            presetId,
            projectId,
            ...overrides,
            signal: current.signal,
            onProgress: (p) => {
              if (controller !== current || current.signal.aborted) return;
              progress.value = p;
              overrides.onProgress?.(p);
            },
          }),
          aborted,
        ]);
      } catch (cause) {
        const err =
          cause instanceof FileBaseError
            ? cause
            : new FileBaseError("unknown", "upload failed", { cause });
        if (controller === current) {
          isUploading.value = false;
          progress.value = null;
          error.value = err;
          onUploadError?.(err);
        }
        return null;
      }

      if (controller === current) {
        isUploading.value = false;
        progress.value = null;
        file.value = result;
        onUploadComplete?.(result);
      }
      return result;
    } finally {
      externalSignal?.removeEventListener("abort", abortFromExternalSignal);
      if (rejectOnAbort) {
        current.signal.removeEventListener("abort", rejectOnAbort);
      }
      if (controller === current) {
        controller = null;
      }
    }
  };

  if (getCurrentScope()) {
    onScopeDispose(abort);
  }

  return { client, isUploading, progress, error, file, upload, abort, reset };
}
