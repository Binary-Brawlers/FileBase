"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type InitializeRequest } from "./api";

export const queryKeys = {
  setupStatus: ["setup", "status"] as const,
  me: ["auth", "me"] as const,
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
