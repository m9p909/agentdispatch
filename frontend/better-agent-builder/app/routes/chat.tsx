import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router";
import { useSession } from "~/hooks/useSessions";
import { useMessages, useStreamChat } from "~/hooks/useMessages";
import { useAgent } from "~/hooks/useAgents";
import type { StreamEvent } from "~/api";

function ToolEventBubble({ event }: { event: StreamEvent }) {
  if (event.type === "tool_call") return (
    <div className="flex justify-start">
      <div className="bg-yellow-50 border border-yellow-200 rounded px-3 py-1 text-xs text-yellow-800">
        🔧 {event.name}({event.arguments})
      </div>
    </div>
  );
  if (event.type === "tool_result") return (
    <div className="flex justify-start">
      <div className="bg-green-50 border border-green-200 rounded px-3 py-1 text-xs text-green-800">
        ✓ {event.result}
      </div>
    </div>
  );
  return null;
}

export default function ChatPage() {
  const { id } = useParams<{ id: string }>();
  const { data: session } = useSession(id!);
  const { data: messages, isLoading } = useMessages(id!);
  const { data: agent } = useAgent(session?.agent_id ?? "");
  const streamChat = useStreamChat(id!);

  const [input, setInput] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamChat.streaming, streamChat.toolEvents]);

  async function handleSend(e: React.FormEvent) {
    e.preventDefault();
    const content = input.trim();
    if (!content) return;
    setInput("");
    await streamChat.send(content);
  }

  return (
    <div className="flex flex-col" style={{ height: "calc(100vh - 120px)" }}>
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold">{session?.title || "Chat"}</h1>
          {agent && <p className="text-sm text-gray-500">Agent: {agent.name}</p>}
        </div>
        <Link to="/" className="text-sm text-gray-500 hover:underline">← Sessions</Link>
      </div>

      <div className="flex-1 overflow-y-auto bg-white border rounded p-4 space-y-4 mb-4">
        {isLoading && <p className="text-gray-400 text-sm">Loading messages...</p>}
        {messages?.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[70%] rounded-lg px-4 py-2 text-sm ${
                msg.role === "user"
                  ? "bg-blue-600 text-white"
                  : "bg-gray-100 text-gray-900"
              }`}
            >
              <p className="text-xs font-semibold mb-1 opacity-70">
                {msg.role === "user" ? "You" : "Agent"}
              </p>
              <p className="whitespace-pre-wrap">{msg.content}</p>
            </div>
          </div>
        ))}
        {streamChat.streaming !== null && (
          <div className="flex justify-start">
            <div className="bg-gray-100 rounded-lg px-4 py-2 text-sm text-gray-900 whitespace-pre-wrap">
              {streamChat.streaming || <span className="animate-pulse text-gray-400">...</span>}
            </div>
          </div>
        )}
        {streamChat.toolEvents.map((e, i) => (
          <ToolEventBubble key={i} event={e} />
        ))}
        <div ref={bottomRef} />
      </div>

      <form onSubmit={handleSend} className="flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Type a message..."
          disabled={streamChat.isPending}
          className="flex-1 border rounded px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          type="submit"
          disabled={streamChat.isPending || !input.trim()}
          className="bg-blue-600 text-white px-5 py-2 rounded hover:bg-blue-700 text-sm disabled:opacity-50"
        >
          Send
        </button>
      </form>
    </div>
  );
}
