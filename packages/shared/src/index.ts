export type FileBaseFile = {
  id: string;
  projectId: string;
  storageConnectionId: string;
  originalName: string;
  savedName: string;
  mimeType: string;
  extension: string;
  size: number;
  hash: string;
  folder: string;
  path: string;
  url: string;
  status: string;
  duplicate: boolean;
  duplicateOfFileId: string | null;
  createdAt: string;
};

export type FileBaseUploadSession = {
  id: string;
  projectId: string;
  presetId: string;
  uploadUrl: string;
  token: string;
  expiresAt: string;
};

export type FileBaseUploadResult = FileBaseFile;

export type FileBaseSignRequest = {
  preset?: string;
  presetId?: string;
  projectId?: string;
  expiresInSeconds?: number;
};

export type FileBaseErrorCode =
  | "sign_failed"
  | "upload_failed"
  | "network_error"
  | "validation_error"
  | "aborted"
  | "unknown";

export class FileBaseError extends Error {
  readonly code: FileBaseErrorCode;
  readonly status?: number;
  readonly details?: unknown;

  constructor(
    code: FileBaseErrorCode,
    message: string,
    options: { status?: number; details?: unknown; cause?: unknown } = {}
  ) {
    super(message);
    this.name = "FileBaseError";
    this.code = code;
    this.status = options.status;
    this.details = options.details;
    if (options.cause !== undefined) {
      (this as { cause?: unknown }).cause = options.cause;
    }
  }
}

export type FileBaseEventName =
  | "file.uploaded"
  | "file.deleted"
  | "file.duplicate_detected"
  | "file.optimized"
  | "file.failed";
