const BASE = "http://localhost:8080/api/v1";

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (res.status === 204) return undefined as T;
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? res.statusText);
  return data as T;
}

export interface Provider {
  id: string;
  name: string;
  type: string;
  base_url?: string;
}

export interface Model {
  id: string;
  provider_id: string;
  name: string;
  model_identifier: string;
}

export interface Agent {
  id: string;
  user_id: string;
  model_id: string;
  parent_id?: string;
  name: string;
  description?: string;
  system_prompt: string;
}

export interface Session {
  id: string;
  agent_id: string;
  user_id: string;
  title?: string;
  is_active: boolean;
}

export interface Message {
  id: string;
  session_id: string;
  role: string;
  content: string;
}

export interface StreamEvent {
  type: "token" | "tool_call" | "tool_result" | "done" | "error";
  delta?: string;
  id?: string;
  name?: string;
  arguments?: string;
  result?: string;
  message_id?: string;
  message?: string;
}

export type CreateProviderData = { name: string; type: string; api_key: string; base_url?: string };
export type UpdateProviderData = CreateProviderData;
export type CreateModelData = { provider_id: string; name: string; model_identifier: string };
export type UpdateModelData = CreateModelData;
export type CreateAgentData = { model_id: string; name: string; description?: string; system_prompt: string };
export type UpdateAgentData = CreateAgentData;
export type CreateSessionData = { agent_id: string; title?: string };

export interface TelegramConnector {
  id: string;
  agent_id: string;
  is_enabled: boolean;
  masked_token: string;
  created_at: string;
}

export type CreateTelegramConnectorData = { agent_id: string; bot_token: string };
export type SetEnabledData = { is_enabled: boolean };
export type AddWhitelistData = { telegram_user_id: number };

export const api = {
  providers: {
    list: () => request<Provider[]>("/providers"),
    get: (id: string) => request<Provider>(`/providers/${id}`),
    create: (data: CreateProviderData) =>
      request<Provider>("/providers", { method: "POST", body: JSON.stringify(data) }),
    update: (id: string, data: UpdateProviderData) =>
      request<Provider>(`/providers/${id}`, { method: "PUT", body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/providers/${id}`, { method: "DELETE" }),
  },
  models: {
    list: () => request<Model[]>("/models"),
    get: (id: string) => request<Model>(`/models/${id}`),
    create: (data: CreateModelData) =>
      request<Model>("/models", { method: "POST", body: JSON.stringify(data) }),
    update: (id: string, data: UpdateModelData) =>
      request<Model>(`/models/${id}`, { method: "PUT", body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/models/${id}`, { method: "DELETE" }),
  },
  agents: {
    list: () => request<Agent[]>("/agents"),
    get: (id: string) => request<Agent>(`/agents/${id}`),
    create: (data: CreateAgentData) =>
      request<Agent>("/agents", { method: "POST", body: JSON.stringify(data) }),
    update: (id: string, data: UpdateAgentData) =>
      request<Agent>(`/agents/${id}`, { method: "PUT", body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/agents/${id}`, { method: "DELETE" }),
  },
  sessions: {
    list: () => request<Session[]>("/sessions"),
    get: (id: string) => request<Session>(`/sessions/${id}`),
    create: (data: CreateSessionData) =>
      request<Session>("/sessions", { method: "POST", body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/sessions/${id}`, { method: "DELETE" }),
  },
  messages: {
    list: (sessionId: string) => request<Message[]>(`/sessions/${sessionId}/messages`),
    send: (sessionId: string, content: string) =>
      request<Message[]>(`/sessions/${sessionId}/messages`, {
        method: "POST",
        body: JSON.stringify({ content }),
      }),
    stream: async function* (
      sessionId: string,
      content: string,
      signal?: AbortSignal
    ): AsyncGenerator<StreamEvent> {
      const { createParser } = await import("eventsource-parser");
      const res = await fetch(`${BASE}/sessions/${sessionId}/stream`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content }),
        signal,
      });
      if (!res.ok || !res.body) throw new Error(await res.text());

      const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
      const events: StreamEvent[] = [];
      const parser = createParser({
        onEvent: (e) => {
          try {
            events.push(JSON.parse(e.data) as StreamEvent);
          } catch {
            events.push({ type: "error", message: `Malformed SSE JSON: ${e.data}` });
          }
        },
      });

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          parser.feed(value);
          yield* events.splice(0);
        }
      } finally {
        reader.releaseLock();
      }
    },
  },
  telegram: {
    list: () => request<TelegramConnector[]>("/connectors/telegram"),
    get: (agentId: string) =>
      request<TelegramConnector | null>(`/connectors/telegram/${agentId}`),
    create: (data: CreateTelegramConnectorData) =>
      request<TelegramConnector>("/connectors/telegram", {
        method: "POST",
        body: JSON.stringify(data),
      }),
    setEnabled: (agentId: string, data: SetEnabledData) =>
      request<TelegramConnector>(`/connectors/telegram/${agentId}`, {
        method: "PATCH",
        body: JSON.stringify(data),
      }),
    delete: (agentId: string) =>
      request<void>(`/connectors/telegram/${agentId}`, { method: "DELETE" }),
    whitelist: {
      list: (agentId: string) =>
        request<number[]>(`/connectors/telegram/${agentId}/whitelist`),
      add: (agentId: string, data: AddWhitelistData) =>
        request<void>(`/connectors/telegram/${agentId}/whitelist`, {
          method: "POST",
          body: JSON.stringify(data),
        }),
      remove: (agentId: string, uid: number) =>
        request<void>(`/connectors/telegram/${agentId}/whitelist/${uid}`, {
          method: "DELETE",
        }),
    },
  },
};
