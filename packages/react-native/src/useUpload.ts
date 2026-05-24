import { useCallback, useMemo, useRef, useState } from "react";
import { FileBaseNativeClient } from "./client";
import { FileBaseError } from "./types";
import type {
  FileBaseNativeClientOptions,
  FileBaseNativeFile,
  NativeUploadOptions,
  NativeUploadProgress,
  UploadFileResult,
} from "./types";

export type UseUploadOptions = FileBaseNativeClientOptions & {
  preset?: string;
  presetId?: string;
  projectId?: string;
  onUploadComplete?: (file: UploadFileResult) => void;
  onUploadError?: (error: FileBaseError) => void;
};

export type UseUploadState = {
  isUploading: boolean;
  progress: NativeUploadProgress | null;
  error: FileBaseError | null;
  file: UploadFileResult | null;
};

export type UseUploadReturn = UseUploadState & {
  client: FileBaseNativeClient;
  upload: (
    file: FileBaseNativeFile,
    overrides?: NativeUploadOptions
  ) => Promise<UploadFileResult | null>;
  abort: () => void;
  reset: () => void;
};

export function useUpload(options: UseUploadOptions): UseUploadReturn {
  const { preset, presetId, projectId, onUploadComplete, onUploadError, signEndpoint, signHeaders } =
    options;
  const client = useMemo(
    () => new FileBaseNativeClient({ signEndpoint, signHeaders }),
    [signEndpoint, JSON.stringify(signHeaders ?? null)]
  );
  const controllerRef = useRef<AbortController | null>(null);
  const [state, setState] = useState<UseUploadState>({
    isUploading: false,
    progress: null,
    error: null,
    file: null,
  });

  const upload = useCallback(
    async (file: FileBaseNativeFile, overrides: NativeUploadOptions = {}) => {
      controllerRef.current?.abort();
      const controller = new AbortController();
      controllerRef.current = controller;
      setState({ isUploading: true, progress: null, error: null, file: null });
      try {
        const result = await client.upload(file, {
          preset,
          presetId,
          projectId,
          ...overrides,
          signal: overrides.signal ?? controller.signal,
          onProgress: (progress) => {
            setState((prev) => ({ ...prev, progress }));
            overrides.onProgress?.(progress);
          },
        });
        setState({ isUploading: false, progress: null, error: null, file: result });
        onUploadComplete?.(result);
        return result;
      } catch (cause) {
        const error =
          cause instanceof FileBaseError
            ? cause
            : new FileBaseError("unknown", "upload failed", { cause });
        setState({ isUploading: false, progress: null, error, file: null });
        onUploadError?.(error);
        return null;
      } finally {
        if (controllerRef.current === controller) {
          controllerRef.current = null;
        }
      }
    },
    [client, preset, presetId, projectId, onUploadComplete, onUploadError]
  );

  const abort = useCallback(() => {
    controllerRef.current?.abort();
    controllerRef.current = null;
  }, []);

  const reset = useCallback(() => {
    controllerRef.current?.abort();
    controllerRef.current = null;
    setState({ isUploading: false, progress: null, error: null, file: null });
  }, []);

  return { ...state, client, upload, abort, reset };
}
