import type {
  FileBaseSignRequest,
  FileBaseUploadResult,
  FileBaseUploadSession,
} from "@binary-brawlers/filebase-shared";

export {
  FileBaseError,
  type FileBaseFile,
  type FileBaseSignRequest,
  type FileBaseUploadResult,
  type FileBaseUploadSession,
} from "@binary-brawlers/filebase-shared";

/** A React Native / Expo file descriptor (e.g. from ImagePicker / DocumentPicker). */
export type FileBaseNativeFile = {
  uri: string;
  name?: string;
  type?: string;
  size?: number;
};

export type FileBaseNativeClientOptions = {
  signEndpoint: string;
  signHeaders?: Record<string, string>;
};

export type NativeUploadProgress = {
  loaded: number;
  total: number;
  fraction: number | null;
};

export type NativeUploadOptions = FileBaseSignRequest & {
  signEndpoint?: string;
  signHeaders?: Record<string, string>;
  filename?: string;
  contentType?: string;
  fields?: Record<string, string>;
  onProgress?: (progress: NativeUploadProgress) => void;
  signal?: AbortSignal;
};

export type UploadFileInput = FileBaseNativeFile &
  NativeUploadOptions & {
    signEndpoint: string;
  };

export type UploadFileResult = FileBaseUploadResult;

export type { FileBaseUploadSession as NativeUploadSession };
