import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState, useRef } from "react";
import { api, type Message, type StreamEvent } from "~/api";

export function useMessages(sessionId: string) {
  return useQuery({
    queryKey: ["messages", sessionId],
    queryFn: () => api.messages.list(sessionId),
    enabled: !!sessionId,
  });
}

export function useStreamChat(sessionId: string) {
  const qc = useQueryClient();
  const [streaming, setStreaming] = useState<string | null>(null);
  const [toolEvents, setToolEvents] = useState<StreamEvent[]>([]);
  const [isPending, setIsPending] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  async function send(content: string) {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    setIsPending(true);
    setStreaming("");
    setToolEvents([]);
    try {
      for await (const event of api.messages.stream(sessionId, content, ctrl.signal)) {
        if (event.type === "token") setStreaming((s) => (s ?? "") + (event.delta ?? ""));
        if (event.type === "tool_call" || event.type === "tool_result") {
          setToolEvents((e) => [...e, event]);
        }
        if (event.type === "done") {
          await qc.invalidateQueries({ queryKey: ["messages", sessionId] });
          setStreaming(null);
          setToolEvents([]);
        }
        if (event.type === "error") throw new Error(event.message);
      }
    } catch (e) {
      if ((e as Error).name !== "AbortError") {
        console.error("Stream error:", e);
      }
    } finally {
      setIsPending(false);
    }
  }

  return { send, streaming, toolEvents, isPending };
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
