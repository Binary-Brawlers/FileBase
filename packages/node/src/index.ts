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

export type UploadFileOptions = FileBaseSignRequest & {
  /** Filename to send in the multipart part. */
  filename?: string;
  /** Content type to send. */
  contentType?: string;
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
    const form = new FormData();
    form.append("file", blob, options.filename ?? "upload.bin");
    if (options.preset) form.append("preset", options.preset);
    if (options.presetId) form.append("preset_id", options.presetId);
    if (options.projectId) form.append("project_id", options.projectId);
    return this.request<FileBaseUploadResult>("POST", "/uploads", { form });
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
    opts: { json?: unknown; form?: FormData; expectJson?: boolean } = {}
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
    } else if (opts.form) {
      body = opts.form;
    }
    let response: Response;
    try {
      response = await this.fetchImpl(url, { method, headers, body });
    } catch (cause) {
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

async function readErrorBody(response: Response): Promise<unknown> {
  const text = await response.text().catch(() => "");
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}
