import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type UpdateProviderData, type CreateProviderData } from "~/api";

export function useProviders() {
  return useQuery({ queryKey: ["providers"], queryFn: api.providers.list });
}

export function useProvider(id: string) {
  return useQuery({ queryKey: ["providers", id], queryFn: () => api.providers.get(id), enabled: !!id });
}

export function useCreateProvider() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateProviderData) => api.providers.create(data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["providers"] }),
  });
}

export function useUpdateProvider() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateProviderData }) =>
      api.providers.update(id, data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["providers"] }),
  });
}

export function useDeleteProvider() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.providers.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["providers"] }),
  });
}
