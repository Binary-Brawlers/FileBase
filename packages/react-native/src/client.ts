import { FileBaseError } from "@binary-brawlers/filebase-shared";
import type {
  FileBaseNativeClientOptions,
  FileBaseNativeFile,
  FileBaseUploadResult,
  FileBaseUploadSession,
  NativeUploadOptions,
  NativeUploadProgress,
} from "./types";

export class FileBaseNativeClient {
  constructor(readonly options: FileBaseNativeClientOptions) {
    if (!options.signEndpoint) {
      throw new FileBaseError("validation_error", "signEndpoint is required");
    }
  }

  async createSession(
    request: NativeUploadOptions = {},
    signEndpointOverride?: string
  ): Promise<FileBaseUploadSession> {
    const endpoint = signEndpointOverride ?? this.options.signEndpoint;
    const { preset, presetId, projectId, expiresInSeconds } = request;
    let response: Response;
    try {
      response = await fetch(endpoint, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          accept: "application/json",
          ...(this.options.signHeaders ?? {}),
          ...(request.signHeaders ?? {}),
        },
        body: JSON.stringify({ preset, presetId, projectId, expiresInSeconds }),
      });
    } catch (cause) {
      throw new FileBaseError("network_error", "failed to reach sign endpoint", { cause });
    }
    if (!response.ok) {
      const details = await readErrorBody(response);
      throw new FileBaseError("sign_failed", "sign endpoint returned an error", {
        status: response.status,
        details,
      });
    }
    const payload = (await response.json().catch(() => null)) as
      | { data?: FileBaseUploadSession }
      | null;
    const session = payload?.data;
    if (!session?.uploadUrl || !session.token) {
      throw new FileBaseError("sign_failed", "sign endpoint returned an invalid session", {
        details: payload,
      });
    }
    return session;
  }

  async upload(
    file: FileBaseNativeFile,
    options: NativeUploadOptions = {}
  ): Promise<FileBaseUploadResult> {
    const session = await this.createSession(options, options.signEndpoint);
    return this.uploadToSession(session, file, options);
  }

  uploadToSession(
    session: FileBaseUploadSession,
    file: FileBaseNativeFile,
    options: NativeUploadOptions = {}
  ): Promise<FileBaseUploadResult> {
    const filename = options.filename ?? file.name ?? guessFilename(file);
    const contentType = options.contentType ?? file.type ?? "application/octet-stream";

    const form = new FormData();
    form.append(
      "file",
      {
        uri: file.uri,
        name: filename,
        type: contentType,
      } as unknown as Blob,
      filename
    );
    for (const [k, v] of Object.entries(options.fields ?? {})) {
      form.append(k, v);
    }
    return xhrUpload(session, form, options.onProgress, options.signal);
  }
}

function xhrUpload(
  session: FileBaseUploadSession,
  body: FormData,
  onProgress?: (progress: NativeUploadProgress) => void,
  signal?: AbortSignal
): Promise<FileBaseUploadResult> {
  return new Promise<FileBaseUploadResult>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", session.uploadUrl, true);
    xhr.setRequestHeader("authorization", `Bearer ${session.token}`);
    if (onProgress && xhr.upload) {
      xhr.upload.onprogress = (event) => {
        onProgress({
          loaded: event.loaded,
          total: event.total,
          fraction: event.lengthComputable && event.total > 0 ? event.loaded / event.total : null,
        });
      };
    }
    xhr.onerror = () => reject(new FileBaseError("network_error", "upload network error"));
    xhr.onabort = () => reject(new FileBaseError("aborted", "upload was aborted"));
    xhr.onload = () => {
      const text = xhr.responseText;
      let payload: { data?: FileBaseUploadResult } | null = null;
      try {
        payload = text ? (JSON.parse(text) as { data?: FileBaseUploadResult }) : null;
      } catch {
        payload = null;
      }
      if (xhr.status < 200 || xhr.status >= 300) {
        reject(
          new FileBaseError("upload_failed", "upload returned an error", {
            status: xhr.status,
            details: payload ?? text,
          })
        );
        return;
      }
      if (!payload?.data) {
        reject(
          new FileBaseError("upload_failed", "upload returned an invalid response", {
            details: payload,
          })
        );
        return;
      }
      resolve(payload.data);
    };
    if (signal) {
      if (signal.aborted) {
        xhr.abort();
        return;
      }
      signal.addEventListener("abort", () => xhr.abort(), { once: true });
    }
    xhr.send(body as unknown as Document);
  });
}

async function readErrorBody(response: Response): Promise<unknown> {
  const text = await response.text().catch(() => "");
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function guessFilename(file: FileBaseNativeFile): string {
  if (file.name) return file.name;
  const fromUri = file.uri.split("/").pop()?.split("?")[0];
  if (fromUri) return fromUri;
  const ext = (file.type || "application/octet-stream").split("/")[1] ?? "bin";
  return `upload.${ext}`;
}
