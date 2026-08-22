import { API_BASE_URL } from "./config";

export type ApiErrorBody = {
  error: { code: string; message: string };
};

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
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
      }
    | {
        type: "s3";
        bucket: string;
        region: string;
        endpoint?: string;
        access_key: string;
        secret_key: string;
        force_path_style?: boolean;
        base_path: string;
        public_base_url: string;
      };
  preset?: {
    name?: string;
    folder?: string;
    allowed_mime_types?: string[];
    allowed_extensions?: string[];
    max_file_size?: number;
    duplicate_strategy?:
      | "return_existing"
      | "upload_new_copy"
      | "reject_duplicate";
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

export type ProjectRole = "owner" | "admin" | "editor" | "viewer";

const PROJECT_ROLE_RANK: Record<ProjectRole, number> = {
  viewer: 0,
  editor: 1,
  admin: 2,
  owner: 3,
};

export function hasProjectRole(
  role: ProjectRole | undefined,
  required: ProjectRole,
) {
  return (
    role !== undefined && PROJECT_ROLE_RANK[role] >= PROJECT_ROLE_RANK[required]
  );
}

export type Project = {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  updated_at: string;
  role: ProjectRole;
};

export type CreateProjectRequest = { name: string; slug?: string };
export type UpdateProjectRequest = { name?: string; slug?: string };

export type StorageConnection = {
  id: string;
  project_id: string;
  type: "local" | "ftp" | "sftp" | "s3";
  host: string | null;
  port: number | null;
  username: string | null;
  has_password: boolean;
  has_private_key: boolean;
  bucket: string | null;
  region: string | null;
  force_path_style: boolean;
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
  bucket?: string;
  region?: string;
  force_path_style?: boolean;
  base_path?: string;
  public_base_url?: string;
};

export type StorageTestResponse = { ok: boolean; message?: string };

export type UploadPreset = {
  id: string;
  project_id: string;
  storage_connection_id: string | null;
  name: string;
  folder: string;
  allowed_mime_types: string[];
  allowed_extensions: string[];
  max_file_size: number;
  duplicate_strategy:
    | "return_existing"
    | "upload_new_copy"
    | "reject_duplicate";
  filename_strategy: string;
  transformations: Record<string, unknown>;
  created_at: string;
  updated_at: string;
};

export type CreateUploadPresetRequest = {
  project_id: string;
  storage_connection_id?: string;
  name: string;
  folder: string;
  allowed_mime_types?: string[];
  allowed_extensions?: string[];
  max_file_size: number;
  duplicate_strategy: UploadPreset["duplicate_strategy"];
  filename_strategy: string;
  transformations?: Record<string, unknown>;
};

export type UpdateUploadPresetRequest = Partial<
  Omit<CreateUploadPresetRequest, "project_id">
>;

export type ApiKey = {
  id: string;
  project_id: string;
  name: string;
  prefix: string;
  last_used_at: string | null;
  created_at: string;
  revoked_at: string | null;
};

export type CreatedApiKey = ApiKey & { secret: string };

export type CreateApiKeyRequest = {
  project_id: string;
  name: string;
  mode: "live" | "test";
};

export type FileRecord = {
  id: string;
  project_id: string;
  storage_connection_id: string;
  original_name: string;
  saved_name: string;
  mime_type: string;
  extension: string;
  size: number;
  hash: string;
  folder: string;
  path: string;
  url: string;
  status: string;
  duplicate_of_file_id: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
};

export type UploadLog = {
  id: string;
  project_id: string;
  file_id: string | null;
  file_name: string | null;
  event: string;
  status: string;
  message: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
};

export type WebhookEvent =
  | "file.uploaded"
  | "file.deleted"
  | "file.duplicate_detected"
  | "file.optimized"
  | "file.failed";

export type Webhook = {
  id: string;
  project_id: string;
  url: string;
  events: WebhookEvent[];
  is_active: boolean;
  created_at: string;
  updated_at: string;
};

export type CreatedWebhook = Webhook & { signing_secret: string };

export type CreateWebhookRequest = {
  project_id: string;
  url: string;
  events: WebhookEvent[];
  secret?: string;
  is_active?: boolean;
};

export type UpdateWebhookRequest = Partial<
  Omit<CreateWebhookRequest, "project_id">
>;

export type WebhookDeliveryLog = {
  id: string;
  webhook_id: string;
  project_id: string;
  file_id: string | null;
  event: string;
  status: string;
  attempt: number;
  status_code: number | null;
  error: string | null;
  request: Record<string, unknown>;
  response: Record<string, unknown>;
  created_at: string;
};

export type FileFilters = {
  project_id?: string;
  search?: string;
  folder?: string;
  mime_type?: string;
  from?: string;
  to?: string;
};

export type FolderRecord = {
  project_id: string;
  path: string;
  name: string;
  parent: string | null;
  file_count: number;
  direct_file_count: number;
  total_size: number;
  preset_count: number;
  latest_upload_at: string | null;
};

export type UploadLogFilters = {
  project_id?: string;
  file_id?: string;
  event?: string;
  status?: string;
  search?: string;
  from?: string;
  to?: string;
  limit?: number;
};

export type AnalyticsFilters = {
  project_id?: string;
  from?: string;
  to?: string;
};

export type AnalyticsBreakdown = {
  key: string;
  count: number;
  bytes: number;
};

export type AnalyticsSummary = {
  scope: {
    project_id: string | null;
    from: string;
    to: string;
  };
  totals: {
    files: number;
    storage_bytes: number;
    projects: number;
    folders: number;
  };
  period: {
    uploads: number;
    uploaded_bytes: number;
    average_file_size: number;
    duplicate_events: number;
    failure_events: number;
    success_rate: number | null;
  };
  trend: { date: string; uploads: number; bytes: number }[];
  mime_types: AnalyticsBreakdown[];
  storage_types: AnalyticsBreakdown[];
  folders: AnalyticsBreakdown[];
  outcomes: { status: string; count: number }[];
};

export type ProjectMember = {
  user_id: string;
  name: string;
  email: string;
  role: ProjectRole;
  joined_at: string;
};

export type ProjectInvitation = {
  id: string;
  project_id: string;
  email: string;
  role: Exclude<ProjectRole, "owner">;
  invited_by: string;
  inviter_name: string;
  expires_at: string;
  created_at: string;
};

export type CreatedProjectInvitation = ProjectInvitation & {
  accept_token: string;
  accept_url: string;
};

export type InvitationPreview = {
  project_name: string;
  email: string;
  role: Exclude<ProjectRole, "owner">;
  expires_at: string;
  existing_account: boolean;
};

export type AcceptedInvitation = {
  token: string;
  user: CurrentUser;
  project_id: string;
  role: Exclude<ProjectRole, "owner">;
};

function queryString(filters?: object) {
  const params = new URLSearchParams();
  Object.entries(filters ?? {}).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== "") {
      params.set(key, String(value));
    }
  });
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

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
  me: (token?: string | null) => request<CurrentUser>("/auth/me", { token }),
  listProjects: (token?: string | null) =>
    request<Project[]>("/projects", { token }),
  createProject: (body: CreateProjectRequest, token?: string | null) =>
    request<Project>("/projects", { method: "POST", body, token }),
  updateProject: (
    id: string,
    body: UpdateProjectRequest,
    token?: string | null,
  ) => request<Project>(`/projects/${id}`, { method: "PATCH", body, token }),
  deleteProject: (id: string, token?: string | null) =>
    request<void>(`/projects/${id}`, { method: "DELETE", token }),
  listProjectMembers: (projectId: string, token?: string | null) =>
    request<ProjectMember[]>(`/projects/${projectId}/members`, { token }),
  updateProjectMember: (
    projectId: string,
    userId: string,
    role: Exclude<ProjectRole, "owner">,
    token?: string | null,
  ) =>
    request<{ role: ProjectRole }>(`/projects/${projectId}/members/${userId}`, {
      method: "PATCH",
      body: { role },
      token,
    }),
  removeProjectMember: (
    projectId: string,
    userId: string,
    token?: string | null,
  ) =>
    request<void>(`/projects/${projectId}/members/${userId}`, {
      method: "DELETE",
      token,
    }),
  listProjectInvitations: (projectId: string, token?: string | null) =>
    request<ProjectInvitation[]>(`/projects/${projectId}/invitations`, {
      token,
    }),
  createProjectInvitation: (
    projectId: string,
    body: { email: string; role: Exclude<ProjectRole, "owner"> },
    token?: string | null,
  ) =>
    request<CreatedProjectInvitation>(`/projects/${projectId}/invitations`, {
      method: "POST",
      body,
      token,
    }),
  revokeProjectInvitation: (
    projectId: string,
    invitationId: string,
    token?: string | null,
  ) =>
    request<void>(`/projects/${projectId}/invitations/${invitationId}`, {
      method: "DELETE",
      token,
    }),
  previewInvitation: (token: string) =>
    request<InvitationPreview>("/team-invitations/preview", {
      method: "POST",
      body: { token },
    }),
  acceptInvitation: (body: {
    token: string;
    name?: string;
    password: string;
  }) =>
    request<AcceptedInvitation>("/team-invitations/accept", {
      method: "POST",
      body,
    }),
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
  listUploadPresets: (token?: string | null) =>
    request<UploadPreset[]>("/upload-presets", { token }),
  createUploadPreset: (
    body: CreateUploadPresetRequest,
    token?: string | null,
  ) =>
    request<UploadPreset>("/upload-presets", { method: "POST", body, token }),
  updateUploadPreset: (
    id: string,
    body: UpdateUploadPresetRequest,
    token?: string | null,
  ) =>
    request<UploadPreset>(`/upload-presets/${id}`, {
      method: "PATCH",
      body,
      token,
    }),
  deleteUploadPreset: (id: string, token?: string | null) =>
    request<void>(`/upload-presets/${id}`, { method: "DELETE", token }),
  listApiKeys: (token?: string | null) =>
    request<ApiKey[]>("/api-keys", { token }),
  createApiKey: (body: CreateApiKeyRequest, token?: string | null) =>
    request<CreatedApiKey>("/api-keys", { method: "POST", body, token }),
  revokeApiKey: (id: string, token?: string | null) =>
    request<ApiKey>(`/api-keys/${id}/revoke`, { method: "PATCH", token }),
  listFiles: (filters?: FileFilters, token?: string | null) =>
    request<FileRecord[]>(`/files${queryString(filters)}`, { token }),
  listFolders: (projectId?: string, token?: string | null) =>
    request<FolderRecord[]>(
      `/folders${queryString({ project_id: projectId })}`,
      { token },
    ),
  getFile: (id: string, token?: string | null) =>
    request<FileRecord>(`/files/${id}`, { token }),
  deleteFile: (id: string, token?: string | null) =>
    request<void>(`/files/${id}`, { method: "DELETE", token }),
  getFileLogs: (id: string, token?: string | null) =>
    request<UploadLog[]>(`/files/${id}/logs`, { token }),
  listUploadLogs: (filters?: UploadLogFilters, token?: string | null) =>
    request<UploadLog[]>(`/upload-logs${queryString(filters)}`, { token }),
  getAnalytics: (filters?: AnalyticsFilters, token?: string | null) =>
    request<AnalyticsSummary>(`/analytics/summary${queryString(filters)}`, {
      token,
    }),
  listWebhooks: (token?: string | null) =>
    request<Webhook[]>("/webhooks", { token }),
  createWebhook: (body: CreateWebhookRequest, token?: string | null) =>
    request<CreatedWebhook>("/webhooks", { method: "POST", body, token }),
  updateWebhook: (
    id: string,
    body: UpdateWebhookRequest,
    token?: string | null,
  ) => request<Webhook>(`/webhooks/${id}`, { method: "PATCH", body, token }),
  deleteWebhook: (id: string, token?: string | null) =>
    request<void>(`/webhooks/${id}`, { method: "DELETE", token }),
  getWebhookDeliveries: (id: string, token?: string | null) =>
    request<WebhookDeliveryLog[]>(`/webhooks/${id}/deliveries`, { token }),
};
