import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  api,
  type TelegramConnector,
  type CreateTelegramConnectorData,
  type SetEnabledData,
  type AddWhitelistData,
} from "~/api";

export function useTelegramConnectors() {
  return useQuery({
    queryKey: ["telegram-connectors"],
    queryFn: api.telegram.list,
  });
}

export function useCreateTelegramConnector() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateTelegramConnectorData) => api.telegram.create(data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["telegram-connectors"] }),
  });
}

export function useSetTelegramEnabled() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ agentId, data }: { agentId: string; data: SetEnabledData }) =>
      api.telegram.setEnabled(agentId, data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["telegram-connectors"] }),
  });
}

export function useDeleteTelegramConnector() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (agentId: string) => api.telegram.delete(agentId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["telegram-connectors"] }),
  });
}

export function useTelegramWhitelist(agentId: string) {
  return useQuery({
    queryKey: ["telegram-whitelist", agentId],
    queryFn: () => api.telegram.whitelist.list(agentId),
    enabled: !!agentId,
  });
}

export function useAddWhitelistEntry(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: AddWhitelistData) => api.telegram.whitelist.add(agentId, data),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["telegram-whitelist", agentId] }),
  });
}

export function useRemoveWhitelistEntry(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (uid: number) => api.telegram.whitelist.remove(agentId, uid),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["telegram-whitelist", agentId] }),
  });
}
