import { useEffect, useReducer, useRef } from "react";
import { type Message, type StreamEvent } from "~/api";
import { useMessages, useStreamChat } from "~/hooks/useMessages";

export type TimelineItem =
  | { kind: "message"; id: string; role: string; content: string }
  | { kind: "tokens"; text: string }
  | { kind: "tool_call"; event: StreamEvent }
  | { kind: "tool_result"; event: StreamEvent }
  | { kind: "error"; message: string };

type ManagerState = {
  timeline: TimelineItem[];
  queue: string[];
  active: boolean;
  seeded: boolean;
  error: string | null;
};

type Action =
  | { type: "SEED"; messages: Message[] }
  | { type: "ENQUEUE"; content: string }
  | { type: "START_STREAM"; content: string; remaining: string[] }
  | { type: "START_STREAM_QUEUE"; remaining: string[] }
  | { type: "STREAM_EVENT"; event: StreamEvent }
  | { type: "DONE" }
  | { type: "ERROR"; message: string };

const initialState: ManagerState = {
  timeline: [],
  queue: [],
  active: false,
  seeded: false,
  error: null,
};

function applyStreamEvent(timeline: TimelineItem[], event: StreamEvent): TimelineItem[] {
  if (event.type === "token") {
    const last = timeline[timeline.length - 1];
    if (last?.kind === "tokens") {
      return [...timeline.slice(0, -1), { kind: "tokens", text: last.text + (event.delta ?? "") }];
    }
    return [...timeline, { kind: "tokens", text: event.delta ?? "" }];
  }
  if (event.type === "tool_call") return [...timeline, { kind: "tool_call", event }];
  if (event.type === "tool_result") return [...timeline, { kind: "tool_result", event }];
  if (event.type === "done") {
    const withoutStreaming = timeline.filter((item) => item.kind !== "tokens" && item.kind !== "tool_call" && item.kind !== "tool_result");
    const tokensItem = timeline.find((item) => item.kind === "tokens") as { kind: "tokens"; text: string } | undefined;
    const content = tokensItem?.text ?? "";
    const id = event.message_id ?? `local-${Date.now()}`;
    return [...withoutStreaming, { kind: "message", id, role: "assistant", content }];
  }
  if (event.type === "error") {
    return [...timeline.filter((item) => item.kind !== "tokens"), { kind: "error", message: event.message ?? "Unknown error" }];
  }
  return timeline;
}

function reducer(state: ManagerState, action: Action): ManagerState {
  switch (action.type) {
    case "SEED": {
      const visible = action.messages.filter(
        (m) => m.role !== "tool" && !(m.role === "assistant" && m.content === "")
      );
      return {
        ...state,
        seeded: true,
        timeline: visible.map((m) => ({ kind: "message", id: m.id, role: m.role, content: m.content })),
      };
    }
    case "ENQUEUE":
      return {
        ...state,
        queue: [...state.queue, action.content],
        timeline: [...state.timeline, { kind: "message", id: `opt-${Date.now()}`, role: "user", content: action.content }],
      };
    case "START_STREAM":
      return {
        ...state,
        active: true,
        queue: action.remaining,
        timeline: [...state.timeline, { kind: "message", id: `opt-${Date.now()}`, role: "user", content: action.content }],
      };
    case "START_STREAM_QUEUE":
      return { ...state, active: true, queue: action.remaining };
    case "STREAM_EVENT":
      return { ...state, timeline: applyStreamEvent(state.timeline, action.event) };
    case "DONE":
      return { ...state, active: false, error: null };
    case "ERROR":
      return { ...state, active: false, error: action.message };
    default:
      return state;
  }
}

export function useMessageManager(sessionId: string) {
  const { data: initialMessages } = useMessages(sessionId);
  const { stream } = useStreamChat(sessionId);
  const [state, dispatch] = useReducer(reducer, initialState);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    if (!state.seeded && initialMessages) {
      dispatch({ type: "SEED", messages: initialMessages });
    }
  }, [initialMessages, state.seeded]);

  function startStream(content: string) {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    void stream(content, (event) => {
      dispatch({ type: "STREAM_EVENT", event });
      if (event.type === "done") dispatch({ type: "DONE" });
      if (event.type === "error") dispatch({ type: "ERROR", message: event.message ?? "Unknown error" });
    }, ctrl.signal);
  }

  useEffect(() => {
    if (!state.active && state.queue.length > 0) {
      const [next, ...rest] = state.queue;
      dispatch({ type: "START_STREAM_QUEUE", remaining: rest });
      startStream(next);
    }
  }, [state.active, state.queue]);

  function send(content: string) {
    if (state.active) {
      dispatch({ type: "ENQUEUE", content });
      return;
    }
    dispatch({ type: "START_STREAM", content, remaining: [] });
    startStream(content);
  }

  return { timeline: state.timeline, send, isPending: state.active, error: state.error };
}
