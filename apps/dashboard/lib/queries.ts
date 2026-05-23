"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  api,
  type CreateApiKeyRequest,
  type CreateProjectRequest,
  type CreateStorageConnectionRequest,
  type CreateUploadPresetRequest,
  type FileFilters,
  type InitializeRequest,
  type UpdateProjectRequest,
  type UpdateStorageConnectionRequest,
  type UpdateUploadPresetRequest,
} from "./api";
import { getToken } from "./auth";

export const queryKeys = {
  setupStatus: ["setup", "status"] as const,
  me: ["auth", "me"] as const,
  projects: ["projects"] as const,
  storageConnections: ["storage-connections"] as const,
  uploadPresets: ["upload-presets"] as const,
  apiKeys: ["api-keys"] as const,
  files: (filters?: FileFilters) => ["files", filters ?? {}] as const,
  fileLogs: (id?: string | null) => ["files", id, "logs"] as const,
};

export function useSetupStatus() {
  return useQuery({
    queryKey: queryKeys.setupStatus,
    queryFn: () => api.getSetupStatus(),
  });
}

export function useMe(token?: string | null) {
  return useQuery({
    queryKey: queryKeys.me,
    queryFn: () => api.me(token),
    enabled: !!token,
  });
}

export function useInitialize() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: InitializeRequest) => api.initialize(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.setupStatus }),
  });
}

export function useLogin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ email, password }: { email: string; password: string }) =>
      api.login(email, password),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.me }),
  });
}

export function useLogout() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.logout(),
    onSuccess: () => qc.removeQueries({ queryKey: queryKeys.me }),
  });
}

export function useProjects() {
  return useQuery({
    queryKey: queryKeys.projects,
    queryFn: () => api.listProjects(getToken()),
  });
}

export function useCreateProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateProjectRequest) => api.createProject(body, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.projects }),
  });
}

export function useUpdateProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateProjectRequest }) =>
      api.updateProject(id, body, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.projects }),
  });
}

export function useDeleteProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteProject(id, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.projects }),
  });
}

export function useStorageConnections() {
  return useQuery({
    queryKey: queryKeys.storageConnections,
    queryFn: () => api.listStorageConnections(getToken()),
  });
}

export function useCreateStorageConnection() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateStorageConnectionRequest) =>
      api.createStorageConnection(body, getToken()),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.storageConnections }),
  });
}

export function useUpdateStorageConnection() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      body,
    }: {
      id: string;
      body: UpdateStorageConnectionRequest;
    }) => api.updateStorageConnection(id, body, getToken()),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.storageConnections }),
  });
}

export function useDeleteStorageConnection() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteStorageConnection(id, getToken()),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: queryKeys.storageConnections }),
  });
}

export function useTestStorageConnection() {
  return useMutation({
    mutationFn: (id: string) => api.testStorageConnection(id, getToken()),
  });
}

export function useUploadPresets() {
  return useQuery({
    queryKey: queryKeys.uploadPresets,
    queryFn: () => api.listUploadPresets(getToken()),
  });
}

export function useCreateUploadPreset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateUploadPresetRequest) => api.createUploadPreset(body, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.uploadPresets }),
  });
}

export function useUpdateUploadPreset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateUploadPresetRequest }) =>
      api.updateUploadPreset(id, body, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.uploadPresets }),
  });
}

export function useDeleteUploadPreset() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteUploadPreset(id, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.uploadPresets }),
  });
}

export function useApiKeys() {
  return useQuery({
    queryKey: queryKeys.apiKeys,
    queryFn: () => api.listApiKeys(getToken()),
  });
}

export function useCreateApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateApiKeyRequest) => api.createApiKey(body, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.apiKeys }),
  });
}

export function useRevokeApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.revokeApiKey(id, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.apiKeys }),
  });
}

export function useFiles(filters?: FileFilters) {
  return useQuery({
    queryKey: queryKeys.files(filters),
    queryFn: () => api.listFiles(filters, getToken()),
  });
}

export function useFileLogs(id?: string | null) {
  return useQuery({
    queryKey: queryKeys.fileLogs(id),
    queryFn: () => api.getFileLogs(id!, getToken()),
    enabled: !!id,
  });
}

export function useDeleteFile() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteFile(id, getToken()),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["files"] }),
  });
}
