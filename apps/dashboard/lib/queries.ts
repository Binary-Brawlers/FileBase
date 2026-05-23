"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  api,
  type CreateStorageConnectionRequest,
  type InitializeRequest,
  type UpdateStorageConnectionRequest,
} from "./api";
import { getToken } from "./auth";

export const queryKeys = {
  setupStatus: ["setup", "status"] as const,
  me: ["auth", "me"] as const,
  projects: ["projects"] as const,
  storageConnections: ["storage-connections"] as const,
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
