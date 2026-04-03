import { useState } from "react";
import { Link } from "react-router";
import { useSessions, useCreateSession, useDeleteSession } from "~/hooks/adapters/useSessions";
import { useAgents } from "~/hooks/adapters/useAgents";

export default function SessionsPage() {
  const { data: sessions, isLoading, error } = useSessions();
  const { data: agents } = useAgents();
  const createSession = useCreateSession();
  const deleteSession = useDeleteSession();

  const [agentId, setAgentId] = useState("");
  const [title, setTitle] = useState("");
  const [showForm, setShowForm] = useState(false);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!agentId) return;
    const session = await createSession.mutateAsync({ agent_id: agentId, title: title || undefined });
    setAgentId("");
    setTitle("");
    setShowForm(false);
    window.location.href = `/sessions/${session.id}/chat`;
  }

  const agentName = (id: string) => agents?.find((a) => a.id === id)?.name ?? id.slice(0, 8);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Sessions</h1>
        <button
          onClick={() => setShowForm(!showForm)}
          className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm"
        >
          {showForm ? "Cancel" : "New Session"}
        </button>
      </div>

      {showForm && (
        <form onSubmit={handleCreate} className="bg-white border rounded p-4 mb-6 flex gap-3 items-end">
          <div className="flex-1">
            <label className="block text-sm font-medium mb-1">Agent</label>
            <select
              value={agentId}
              onChange={(e) => setAgentId(e.target.value)}
              required
              className="w-full border rounded px-3 py-2 text-sm"
            >
              <option value="">Select an agent...</option>
              {agents?.map((a) => (
                <option key={a.id} value={a.id}>{a.name}</option>
              ))}
            </select>
          </div>
          <div className="flex-1">
            <label className="block text-sm font-medium mb-1">Title (optional)</label>
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Chat title..."
              className="w-full border rounded px-3 py-2 text-sm"
            />
          </div>
          <button
            type="submit"
            disabled={createSession.isPending}
            className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm disabled:opacity-50"
          >
            {createSession.isPending ? "Creating..." : "Start Chat"}
          </button>
        </form>
      )}

      {isLoading && <p className="text-gray-500">Loading...</p>}
      {error && <p className="text-red-500">Error: {error.message}</p>}

      {sessions && sessions.length === 0 && (
        <p className="text-gray-500">No sessions yet. Start a new one above.</p>
      )}

      {sessions && sessions.length > 0 && (
        <table className="w-full bg-white border rounded">
          <thead>
            <tr className="border-b bg-gray-50 text-left text-sm text-gray-600">
              <th className="px-4 py-3">Title</th>
              <th className="px-4 py-3">Agent</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3"></th>
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => (
              <tr key={s.id} className="border-b last:border-0 hover:bg-gray-50">
                <td className="px-4 py-3 text-sm">{s.title || "Untitled"}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{agentName(s.agent_id)}</td>
                <td className="px-4 py-3">
                  <span className={`text-xs px-2 py-0.5 rounded-full ${s.is_active ? "bg-green-100 text-green-700" : "bg-gray-100 text-gray-600"}`}>
                    {s.is_active ? "active" : "ended"}
                  </span>
                </td>
                <td className="px-4 py-3 text-right space-x-2">
                  <Link
                    to={`/sessions/${s.id}/chat`}
                    className="text-sm text-blue-600 hover:underline"
                  >
                    Open
                  </Link>
                  <button
                    onClick={() => deleteSession.mutate(s.id)}
                    className="text-sm text-red-500 hover:underline"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
