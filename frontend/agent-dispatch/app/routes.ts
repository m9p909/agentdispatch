import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/sessions.tsx"),
  route("sessions/:id/chat", "routes/chat.tsx"),
  route("providers", "routes/providers.tsx"),
  route("models", "routes/models.tsx"),
  route("agents", "routes/agents.tsx"),
  route("connectors", "routes/connectors.tsx"),
] satisfies RouteConfig;
