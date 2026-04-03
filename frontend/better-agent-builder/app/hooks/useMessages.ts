import { useQuery } from "@tanstack/react-query";
import { api, type StreamEvent } from "~/api";

export function useMessages(sessionId: string) {
  return useQuery({
    queryKey: ["messages", sessionId],
    queryFn: () => api.messages.list(sessionId),
    enabled: !!sessionId,
  });
}

export function useStreamChat(sessionId: string) {
  async function stream(content: string, onEvent: (e: StreamEvent) => void, signal: AbortSignal): Promise<void> {
    try {
      for await (const event of api.messages.stream(sessionId, content, signal)) {
        onEvent(event);
      }
    } catch (e) {
      if ((e as Error).name !== "AbortError")
        onEvent({ type: "error", message: (e as Error).message });
    }
  }

  return { stream };
}

export { useMessageManager, type TimelineItem } from "~/hooks/manager/useMessageManager";
