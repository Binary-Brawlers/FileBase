import {
  FileBaseError,
  type FileBaseFile,
  type FileBaseSignRequest,
  type FileBaseUploadResult,
  type FileBaseUploadSession,
} from "@binary-brawlers/filebase-shared";

export {
  FileBaseError,
  type FileBaseFile,
  type FileBaseSignRequest,
  type FileBaseUploadResult,
  type FileBaseUploadSession,
};

export type FileBaseOptions = {
  /** Base URL of the FileBase API (e.g. https://uploads.example.com). */
  gatewayUrl: string;
  /** API key with `fb_live_` or `fb_test_` prefix. */
  apiKey: string;
  /** Optional fetch override. */
  fetch?: typeof fetch;
};

export type CreateUploadSessionOptions = FileBaseSignRequest;

export type UploadProgress = {
  loaded: number;
  total: number;
  /** 0..1, or `null` when total is unknown. */
  fraction: number | null;
};

export type UploadFileOptions = FileBaseSignRequest & {
  /** Filename to send in the multipart part. */
  filename?: string;
  /** Content type to send. */
  contentType?: string;
  /** Progress callback fired as bytes are written to the network. */
  onProgress?: (progress: UploadProgress) => void;
  /** AbortSignal to cancel the upload. */
  signal?: AbortSignal;
};

export type ListFilesQuery = {
  projectId?: string;
  search?: string;
  mimeType?: string;
  from?: string;
  to?: string;
};

export class FileBase {
  readonly gatewayUrl: string;
  readonly apiKey: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: FileBaseOptions) {
    if (!options.apiKey) {
      throw new FileBaseError("validation_error", "apiKey is required");
    }
    if (!options.gatewayUrl) {
      throw new FileBaseError("validation_error", "gatewayUrl is required");
    }
    const fetchImpl = options.fetch ?? globalThis.fetch;
    if (!fetchImpl) {
      throw new FileBaseError("network_error", "fetch is not available in this environment");
    }
    this.gatewayUrl = options.gatewayUrl.replace(/\/+$/, "");
    this.apiKey = options.apiKey;
    this.fetchImpl = fetchImpl;
  }

  async createUploadSession(options: CreateUploadSessionOptions = {}): Promise<FileBaseUploadSession> {
    const data = await this.request<FileBaseUploadSession>("POST", "/uploads/sign", {
      json: snakeKeys(options),
    });
    return data;
  }

  /** Upload bytes directly from the server using the API key (no signed session). */
  async uploadFile(
    file: Blob | ArrayBuffer | Uint8Array,
    options: UploadFileOptions = {}
  ): Promise<FileBaseUploadResult> {
    const blob = toBlob(file, options.contentType);
    const fields: Record<string, string> = {};
    if (options.preset) fields.preset = options.preset;
    if (options.presetId) fields.preset_id = options.presetId;
    if (options.projectId) fields.project_id = options.projectId;

    if (options.onProgress) {
      const { body, contentType } = await buildMultipart(
        blob,
        options.filename ?? "upload.bin",
        fields
      );
      return this.request<FileBaseUploadResult>("POST", "/uploads", {
        rawBody: progressStream(body, options.onProgress),
        rawContentType: contentType,
        contentLength: body.byteLength,
        signal: options.signal,
      });
    }

    const form = new FormData();
    form.append("file", blob, options.filename ?? "upload.bin");
    for (const [k, v] of Object.entries(fields)) form.append(k, v);
    return this.request<FileBaseUploadResult>("POST", "/uploads", {
      form,
      signal: options.signal,
    });
  }

  async listFiles(query: ListFilesQuery = {}): Promise<FileBaseFile[]> {
    return this.request<FileBaseFile[]>("GET", `/files${buildQuery(query)}`);
  }

  async getFile(id: string): Promise<FileBaseFile> {
    return this.request<FileBaseFile>("GET", `/files/${encodeURIComponent(id)}`);
  }

  async deleteFile(id: string): Promise<void> {
    await this.request<null>("DELETE", `/files/${encodeURIComponent(id)}`, { expectJson: false });
  }

  private async request<T>(
    method: string,
    path: string,
    opts: {
      json?: unknown;
      form?: FormData;
      rawBody?: ReadableStream<Uint8Array>;
      rawContentType?: string;
      contentLength?: number;
      signal?: AbortSignal;
      expectJson?: boolean;
    } = {}
  ): Promise<T> {
    const url = `${this.gatewayUrl}${path}`;
    const headers: Record<string, string> = {
      authorization: `Bearer ${this.apiKey}`,
      "x-api-key": this.apiKey,
      accept: "application/json",
    };
    let body: BodyInit | undefined;
    if (opts.json !== undefined) {
      headers["content-type"] = "application/json";
      body = JSON.stringify(opts.json);
    } else if (opts.rawBody) {
      if (opts.rawContentType) headers["content-type"] = opts.rawContentType;
      if (opts.contentLength !== undefined) {
        headers["content-length"] = String(opts.contentLength);
      }
      body = opts.rawBody;
    } else if (opts.form) {
      body = opts.form;
    }
    let response: Response;
    try {
      const init: RequestInit & { duplex?: "half" } = {
        method,
        headers,
        body,
        signal: opts.signal,
      };
      if (opts.rawBody) init.duplex = "half";
      response = await this.fetchImpl(url, init);
    } catch (cause) {
      if (opts.signal?.aborted) {
        throw new FileBaseError("aborted", `request to ${path} was aborted`, { cause });
      }
      throw new FileBaseError("network_error", `request to ${path} failed`, { cause });
    }
    if (!response.ok) {
      const details = await readErrorBody(response);
      throw new FileBaseError("upload_failed", `request to ${path} failed`, {
        status: response.status,
        details,
      });
    }
    if (opts.expectJson === false) {
      return undefined as T;
    }
    const payload = (await response.json().catch(() => null)) as { data?: T } | null;
    if (payload?.data === undefined) {
      throw new FileBaseError("upload_failed", `response for ${path} had no data field`, {
        details: payload,
      });
    }
    return payload.data;
  }
}

function toBlob(input: Blob | ArrayBuffer | Uint8Array, contentType?: string): Blob {
  if (typeof Blob !== "undefined" && input instanceof Blob) {
    return contentType ? new Blob([input], { type: contentType }) : input;
  }
  const source =
    input instanceof Uint8Array ? input : new Uint8Array(input as ArrayBuffer);
  const buffer = new ArrayBuffer(source.byteLength);
  new Uint8Array(buffer).set(source);
  return new Blob([buffer], { type: contentType ?? "application/octet-stream" });
}

function snakeKeys(value: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(value)) {
    if (v === undefined) continue;
    out[k.replace(/[A-Z]/g, (m) => `_${m.toLowerCase()}`)] = v;
  }
  return out;
}

function buildQuery(query: Record<string, string | undefined>): string {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) {
    if (v === undefined || v === "") continue;
    params.set(k.replace(/[A-Z]/g, (m) => `_${m.toLowerCase()}`), v);
  }
  const s = params.toString();
  return s ? `?${s}` : "";
}

async function buildMultipart(
  blob: Blob,
  filename: string,
  fields: Record<string, string>
): Promise<{ body: Uint8Array; contentType: string }> {
  const boundary = `----FileBaseBoundary${Math.random().toString(36).slice(2)}${Date.now().toString(36)}`;
  const enc = new TextEncoder();
  const parts: Uint8Array[] = [];
  for (const [k, v] of Object.entries(fields)) {
    parts.push(
      enc.encode(
        `--${boundary}\r\nContent-Disposition: form-data; name="${k}"\r\n\r\n${v}\r\n`
      )
    );
  }
  parts.push(
    enc.encode(
      `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${filename.replace(/"/g, "%22")}"\r\nContent-Type: ${blob.type || "application/octet-stream"}\r\n\r\n`
    )
  );
  parts.push(new Uint8Array(await blob.arrayBuffer()));
  parts.push(enc.encode(`\r\n--${boundary}--\r\n`));

  const total = parts.reduce((n, p) => n + p.byteLength, 0);
  const body = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    body.set(p, offset);
    offset += p.byteLength;
  }
  return { body, contentType: `multipart/form-data; boundary=${boundary}` };
}

function progressStream(
  body: Uint8Array,
  onProgress: (progress: UploadProgress) => void,
  chunkSize = 64 * 1024
): ReadableStream<Uint8Array> {
  let offset = 0;
  const total = body.byteLength;
  return new ReadableStream<Uint8Array>({
    pull(controller) {
      if (offset >= total) {
        controller.close();
        return;
      }
      const end = Math.min(offset + chunkSize, total);
      controller.enqueue(body.subarray(offset, end));
      offset = end;
      onProgress({
        loaded: offset,
        total,
        fraction: total > 0 ? offset / total : null,
      });
    },
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
