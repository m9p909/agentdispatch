import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type Message } from "~/api";

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
    onMutate: async (content: string) => {
      const previous = qc.getQueryData<Message[]>(["messages", sessionId]);
      const optimistic: Message = {
        id: `temp-${Date.now()}`,
        session_id: sessionId,
        role: "user",
        content,
      };
      qc.setQueryData<Message[]>(["messages", sessionId], (old = []) => [...old, optimistic]);
      return { previous };
    },
    onSuccess: (messages) => {
      qc.setQueryData(["messages", sessionId], messages);
    },
    onError: (_err, _content, context) => {
      if (context?.previous) {
        qc.setQueryData(["messages", sessionId], context.previous);
      }
    },
  });
}
