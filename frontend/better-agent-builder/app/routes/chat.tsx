import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router";
import { useSession } from "~/hooks/adapters/useSessions";
import { useMessageManager, type TimelineItem } from "~/hooks/useMessages";
import { useAgent } from "~/hooks/adapters/useAgents";

function TimelineBubble({ item }: { item: TimelineItem }) {
  if (item.kind === "message") return (
    <div className={`flex ${item.role === "user" ? "justify-end" : "justify-start"}`}>
      <div className={`max-w-[70%] rounded-lg px-4 py-2 text-sm ${
        item.role === "user" ? "bg-blue-600 text-white" : "bg-gray-100 text-gray-900"
      }`}>
        <p className="text-xs font-semibold mb-1 opacity-70">{item.role === "user" ? "You" : "Agent"}</p>
        <p className="whitespace-pre-wrap">{item.content}</p>
      </div>
    </div>
  );
  if (item.kind === "tokens") return (
    <div className="flex justify-start">
      <div className="bg-gray-100 rounded-lg px-4 py-2 text-sm text-gray-900 whitespace-pre-wrap">
        {item.text || <span className="animate-pulse text-gray-400">...</span>}
      </div>
    </div>
  );
  if (item.kind === "tool_call") return (
    <div className="flex justify-start">
      <div className="bg-yellow-50 border border-yellow-200 rounded px-3 py-1 text-xs text-yellow-800">
        🔧 {item.event.name}({item.event.arguments})
      </div>
    </div>
  );
  if (item.kind === "tool_result") return (
    <div className="flex justify-start">
      <div className="bg-green-50 border border-green-200 rounded px-3 py-1 text-xs text-green-800">
        ✓ {item.event.result}
      </div>
    </div>
  );
  if (item.kind === "error") return (
    <div className="flex justify-start">
      <div className="max-w-[70%] rounded-lg px-4 py-2 text-sm bg-red-50 border border-red-200 text-red-700">
        <p className="text-xs font-semibold mb-1">Error</p>
        <p className="whitespace-pre-wrap">{item.message}</p>
      </div>
    </div>
  );
  return null;
}

export default function ChatPage() {
  const { id } = useParams<{ id: string }>();
  const { data: session } = useSession(id!);
  const { data: agent } = useAgent(session?.agent_id ?? "");
  const { timeline, send, isPending } = useMessageManager(id!);

  const [input, setInput] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [timeline]);

  function handleSend(e: React.FormEvent) {
    e.preventDefault();
    const content = input.trim();
    if (!content) return;
    setInput("");
    send(content);
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
        {timeline.length === 0 && <p className="text-gray-400 text-sm">Loading messages...</p>}
        {timeline.map((item, i) => (
          <TimelineBubble key={item.kind === "message" ? item.id : i} item={item} />
        ))}
        <div ref={bottomRef} />
      </div>

      <form onSubmit={handleSend} className="flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Type a message..."
          className="flex-1 border rounded px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          type="submit"
          disabled={!input.trim()}
          className="bg-blue-600 text-white px-5 py-2 rounded hover:bg-blue-700 text-sm disabled:opacity-50"
        >
          Send
        </button>
      </form>
    </div>
  );
}
