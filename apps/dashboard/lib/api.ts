import { API_BASE_URL } from "./config";

export type ApiErrorBody = {
  error: { code: string; message: string };
};

export class ApiError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
    this.name = "ApiError";
  }
}

type RequestOptions = {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  body?: unknown;
  token?: string | null;
  cache?: RequestCache;
};

async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  if (opts.body !== undefined) headers["Content-Type"] = "application/json";
  if (opts.token) headers["Authorization"] = `Bearer ${opts.token}`;

  const res = await fetch(`${API_BASE_URL}${path}`, {
    method: opts.method ?? "GET",
    headers,
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
    credentials: "include",
    cache: opts.cache ?? "no-store",
  });

  if (res.status === 204) return undefined as T;

  const text = await res.text();
  const parsed = text ? JSON.parse(text) : null;

  if (!res.ok) {
    const err = (parsed as ApiErrorBody | null)?.error;
    throw new ApiError(
      res.status,
      err?.code ?? "unknown",
      err?.message ?? res.statusText,
    );
  }

  return (parsed?.data ?? parsed) as T;
}

export type SetupStatus = { setup_required: boolean };

export type InitializeRequest = {
  admin: { name: string; email: string; password: string };
  project: { name: string; slug?: string };
  storage:
    | { type: "local"; base_path: string; public_base_url: string }
    | {
        type: "ftp";
        host: string;
        port?: number;
        username: string;
        password: string;
        base_path: string;
        public_base_url: string;
      }
    | {
        type: "sftp";
        host: string;
        port?: number;
        username: string;
        password?: string;
        private_key?: string;
        base_path: string;
        public_base_url: string;
      };
  preset?: {
    name?: string;
    folder?: string;
    allowed_mime_types?: string[];
    allowed_extensions?: string[];
    max_file_size?: number;
    duplicate_strategy?: "return_existing" | "upload_new_copy" | "reject_duplicate";
    filename_strategy?: string;
    transformations?: Record<string, unknown>;
  };
};

export type InitializeResponse = {
  user_id: string;
  project_id: string;
  storage_connection_id: string;
  upload_preset_id: string;
};

export type LoginResponse = {
  token: string;
  user: { id: string; name: string; email: string };
};

export type CurrentUser = { id: string; name: string; email: string };

export type Project = {
  id: string;
  name: string;
  slug: string;
  created_at: string;
};

export type StorageConnection = {
  id: string;
  project_id: string;
  type: "local" | "ftp" | "sftp";
  host: string | null;
  port: number | null;
  username: string | null;
  has_password: boolean;
  has_private_key: boolean;
  base_path: string;
  public_base_url: string;
  created_at: string;
  updated_at: string;
};

export type CreateStorageConnectionRequest = {
  project_id: string;
} & InitializeRequest["storage"];

export type UpdateStorageConnectionRequest = {
  host?: string;
  port?: number;
  username?: string;
  password?: string;
  private_key?: string;
  base_path?: string;
  public_base_url?: string;
};

export type StorageTestResponse = { ok: boolean; message?: string };

export const api = {
  getSetupStatus: (opts?: { token?: string }) =>
    request<SetupStatus>("/setup/status", { token: opts?.token }),
  initialize: (body: InitializeRequest) =>
    request<InitializeResponse>("/setup/initialize", { method: "POST", body }),
  login: (email: string, password: string) =>
    request<LoginResponse>("/auth/login", {
      method: "POST",
      body: { email, password },
    }),
  logout: () => request<void>("/auth/logout", { method: "POST" }),
  me: (token?: string | null) =>
    request<CurrentUser>("/auth/me", { token }),
  listProjects: (token?: string | null) =>
    request<Project[]>("/projects", { token }),
  listStorageConnections: (token?: string | null) =>
    request<StorageConnection[]>("/storage-connections", { token }),
  createStorageConnection: (
    body: CreateStorageConnectionRequest,
    token?: string | null,
  ) =>
    request<StorageConnection>("/storage-connections", {
      method: "POST",
      body,
      token,
    }),
  updateStorageConnection: (
    id: string,
    body: UpdateStorageConnectionRequest,
    token?: string | null,
  ) =>
    request<StorageConnection>(`/storage-connections/${id}`, {
      method: "PATCH",
      body,
      token,
    }),
  deleteStorageConnection: (id: string, token?: string | null) =>
    request<void>(`/storage-connections/${id}`, { method: "DELETE", token }),
  testStorageConnection: (id: string, token?: string | null) =>
    request<StorageTestResponse>(`/storage-connections/${id}/test`, {
      method: "POST",
      token,
    }),
};
