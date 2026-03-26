import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "~/api";

export function useMessages(sessionId: string) {
  return useQuery({
    queryKey: ["messages", sessionId],
    queryFn: () => api.messages.list(sessionId),
    enabled: !!sessionId,
  });
}

export function useSendMessage(sessionId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (content: string) => api.messages.send(sessionId, content),
    onSuccess: (messages) => {
      qc.setQueryData(["messages", sessionId], messages);
    },
  });
}
