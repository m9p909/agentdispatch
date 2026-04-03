import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type CreateModelData, type UpdateModelData } from "~/api";

export function useModels() {
  return useQuery({ queryKey: ["models"], queryFn: api.models.list });
}

export function useModel(id: string) {
  return useQuery({ queryKey: ["models", id], queryFn: () => api.models.get(id), enabled: !!id });
}

export function useCreateModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateModelData) => api.models.create(data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["models"] }),
  });
}

export function useUpdateModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateModelData }) =>
      api.models.update(id, data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["models"] }),
  });
}

export function useDeleteModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.models.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["models"] }),
  });
}
