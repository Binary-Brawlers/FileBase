import {
  FileBaseError,
  type FileBaseFile,
  type FileBaseSignRequest,
  type FileBaseUploadResult,
  type FileBaseUploadSession,
} from "@filebase/shared";

export {
  FileBaseError,
  type FileBaseFile,
  type FileBaseSignRequest,
  type FileBaseUploadResult,
  type FileBaseUploadSession,
};

export type FileBaseClientOptions = {
  /** URL of the developer's backend route that returns a signed upload session. */
  signEndpoint: string;
  /** Optional fetch override (useful for tests or custom credentials). */
  fetch?: typeof fetch;
  /** Optional extra headers to send to the sign endpoint. */
  signHeaders?: Record<string, string>;
  /** Optional credentials mode for the sign endpoint fetch. Defaults to "same-origin". */
  signCredentials?: RequestCredentials;
};

export type UploadProgress = {
  loaded: number;
  total: number;
  /** 0..1, or `null` when total is unknown. */
  fraction: number | null;
};

export type UploadOptions = FileBaseSignRequest & {
  /** Optional override for the filename sent with the multipart part. */
  filename?: string;
  /** Optional override for the multipart content-type. */
  contentType?: string;
  /** Progress callback for upload bytes (browser only). */
  onProgress?: (progress: UploadProgress) => void;
  /** AbortSignal for cancelling the upload. */
  signal?: AbortSignal;
  /** Additional multipart fields to include. */
  fields?: Record<string, string>;
};

export class FileBaseClient {
  constructor(readonly options: FileBaseClientOptions) {
    if (!options.signEndpoint) {
      throw new FileBaseError("validation_error", "signEndpoint is required");
    }
  }

  /** Request a signed upload session from the configured sign endpoint. */
  async createSession(request: FileBaseSignRequest = {}): Promise<FileBaseUploadSession> {
    const fetchImpl = this.options.fetch ?? globalThis.fetch;
    if (!fetchImpl) {
      throw new FileBaseError("network_error", "fetch is not available in this environment");
    }
    let response: Response;
    try {
      response = await fetchImpl(this.options.signEndpoint, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          accept: "application/json",
          ...(this.options.signHeaders ?? {}),
        },
        credentials: this.options.signCredentials ?? "same-origin",
        body: JSON.stringify(request),
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

  /** Sign and upload a single file in one call. */
  async upload(file: Blob, options: UploadOptions = {}): Promise<FileBaseUploadResult> {
    const { onProgress, signal, filename, contentType, fields, ...sign } = options;
    const session = await this.createSession(sign);
    return this.uploadToSession(session, file, { onProgress, signal, filename, contentType, fields });
  }

  /** Upload to a pre-existing session (e.g. one created by your backend). */
  async uploadToSession(
    session: FileBaseUploadSession,
    file: Blob,
    options: {
      onProgress?: (progress: UploadProgress) => void;
      signal?: AbortSignal;
      filename?: string;
      contentType?: string;
      fields?: Record<string, string>;
    } = {}
  ): Promise<FileBaseUploadResult> {
    const form = new FormData();
    const fileName = options.filename ?? guessFilename(file);
    if (options.contentType && file instanceof Blob) {
      form.append("file", new Blob([file], { type: options.contentType }), fileName);
    } else {
      form.append("file", file, fileName);
    }
    for (const [k, v] of Object.entries(options.fields ?? {})) {
      form.append(k, v);
    }

    if (typeof XMLHttpRequest !== "undefined" && options.onProgress) {
      return xhrUpload(session, form, options.onProgress, options.signal);
    }
    return fetchUpload(this.options.fetch ?? globalThis.fetch, session, form, options.signal);
  }
}

async function fetchUpload(
  fetchImpl: typeof fetch | undefined,
  session: FileBaseUploadSession,
  body: FormData,
  signal?: AbortSignal
): Promise<FileBaseUploadResult> {
  if (!fetchImpl) {
    throw new FileBaseError("network_error", "fetch is not available in this environment");
  }
  let response: Response;
  try {
    response = await fetchImpl(session.uploadUrl, {
      method: "POST",
      headers: { authorization: `Bearer ${session.token}` },
      body,
      signal,
    });
  } catch (cause) {
    if (signal?.aborted) {
      throw new FileBaseError("aborted", "upload was aborted", { cause });
    }
    throw new FileBaseError("network_error", "upload network error", { cause });
  }
  if (!response.ok) {
    const details = await readErrorBody(response);
    throw new FileBaseError("upload_failed", "upload returned an error", {
      status: response.status,
      details,
    });
  }
  const payload = (await response.json().catch(() => null)) as { data?: FileBaseFile } | null;
  if (!payload?.data) {
    throw new FileBaseError("upload_failed", "upload returned an invalid response", {
      details: payload,
    });
  }
  return payload.data;
}

function xhrUpload(
  session: FileBaseUploadSession,
  body: FormData,
  onProgress: (progress: UploadProgress) => void,
  signal?: AbortSignal
): Promise<FileBaseUploadResult> {
  return new Promise<FileBaseUploadResult>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", session.uploadUrl, true);
    xhr.setRequestHeader("authorization", `Bearer ${session.token}`);
    xhr.upload.onprogress = (event) => {
      onProgress({
        loaded: event.loaded,
        total: event.total,
        fraction: event.lengthComputable && event.total > 0 ? event.loaded / event.total : null,
      });
    };
    xhr.onerror = () => {
      reject(new FileBaseError("network_error", "upload network error"));
    };
    xhr.onabort = () => {
      reject(new FileBaseError("aborted", "upload was aborted"));
    };
    xhr.onload = () => {
      const text = xhr.responseText;
      let payload: { data?: FileBaseFile; error?: unknown } | null = null;
      try {
        payload = text ? (JSON.parse(text) as { data?: FileBaseFile; error?: unknown }) : null;
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
    xhr.send(body);
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

function guessFilename(file: Blob): string {
  if (typeof File !== "undefined" && file instanceof File && file.name) {
    return file.name;
  }
  const ext = (file.type || "application/octet-stream").split("/")[1] ?? "bin";
  return `upload.${ext}`;
}
