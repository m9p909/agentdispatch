import { useState } from "react";
import { useAgents, useCreateAgent, useUpdateAgent, useDeleteAgent } from "~/hooks/useAgents";
import { useModels } from "~/hooks/useModels";
import type { Agent, CreateAgentData } from "~/api";

type FormState = { model_id: string; name: string; description: string; system_prompt: string };
const empty: FormState = { model_id: "", name: "", description: "", system_prompt: "" };

export default function AgentsPage() {
  const { data: agents, isLoading, error } = useAgents();
  const { data: models } = useModels();
  const createAgent = useCreateAgent();
  const updateAgent = useUpdateAgent();
  const deleteAgent = useDeleteAgent();

  const [form, setForm] = useState<FormState>(empty);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  function handleEdit(a: Agent) {
    setEditingId(a.id);
    setForm({
      model_id: a.model_id,
      name: a.name,
      description: a.description ?? "",
      system_prompt: a.system_prompt,
    });
    setShowForm(true);
  }

  function handleCancel() {
    setForm(empty);
    setEditingId(null);
    setShowForm(false);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const data: CreateAgentData = {
      model_id: form.model_id,
      name: form.name,
      description: form.description || undefined,
      system_prompt: form.system_prompt,
    };
    if (editingId) {
      await updateAgent.mutateAsync({ id: editingId, data });
    } else {
      await createAgent.mutateAsync(data);
    }
    handleCancel();
  }

  const isPending = createAgent.isPending || updateAgent.isPending;
  const mutateError = createAgent.error ?? updateAgent.error;
  const modelName = (id: string) => models?.find((m) => m.id === id)?.name ?? id.slice(0, 8);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Agents</h1>
        {!showForm && (
          <button
            onClick={() => setShowForm(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm"
          >
            Add Agent
          </button>
        )}
      </div>

      {showForm && (
        <form onSubmit={handleSubmit} className="bg-white border rounded p-4 mb-6 space-y-3">
          <h2 className="font-semibold">{editingId ? "Edit Agent" : "New Agent"}</h2>
          {mutateError && <p className="text-red-500 text-sm">{mutateError.message}</p>}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1">Name</label>
              <input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                required
                className="w-full border rounded px-3 py-2 text-sm"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Model</label>
              <select
                value={form.model_id}
                onChange={(e) => setForm({ ...form, model_id: e.target.value })}
                required
                className="w-full border rounded px-3 py-2 text-sm"
              >
                <option value="">Select a model...</option>
                {models?.map((m) => (
                  <option key={m.id} value={m.id}>{m.name}</option>
                ))}
              </select>
            </div>
            <div className="col-span-2">
              <label className="block text-sm font-medium mb-1">Description (optional)</label>
              <input
                value={form.description}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
                className="w-full border rounded px-3 py-2 text-sm"
              />
            </div>
            <div className="col-span-2">
              <label className="block text-sm font-medium mb-1">System Prompt</label>
              <textarea
                value={form.system_prompt}
                onChange={(e) => setForm({ ...form, system_prompt: e.target.value })}
                rows={4}
                required
                className="w-full border rounded px-3 py-2 text-sm"
              />
            </div>
          </div>
          <div className="flex gap-2">
            <button
              type="submit"
              disabled={isPending}
              className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm disabled:opacity-50"
            >
              {isPending ? "Saving..." : editingId ? "Update" : "Create"}
            </button>
            <button type="button" onClick={handleCancel} className="text-sm text-gray-500 hover:underline">
              Cancel
            </button>
          </div>
        </form>
      )}

      {isLoading && <p className="text-gray-500">Loading...</p>}
      {error && <p className="text-red-500">Error: {error.message}</p>}

      {agents && agents.length === 0 && <p className="text-gray-500">No agents yet.</p>}

      {agents && agents.length > 0 && (
        <table className="w-full bg-white border rounded">
          <thead>
            <tr className="border-b bg-gray-50 text-left text-sm text-gray-600">
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Model</th>
              <th className="px-4 py-3">Description</th>
              <th className="px-4 py-3"></th>
            </tr>
          </thead>
          <tbody>
            {agents.map((a) => (
              <tr key={a.id} className="border-b last:border-0 hover:bg-gray-50">
                <td className="px-4 py-3 text-sm font-medium">{a.name}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{modelName(a.model_id)}</td>
                <td className="px-4 py-3 text-sm text-gray-500">{a.description ?? "—"}</td>
                <td className="px-4 py-3 text-right space-x-2">
                  <button onClick={() => handleEdit(a)} className="text-sm text-blue-600 hover:underline">Edit</button>
                  <button
                    onClick={() => deleteAgent.mutate(a.id)}
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
