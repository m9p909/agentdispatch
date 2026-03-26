import { useState } from "react";
import { useProviders, useCreateProvider, useUpdateProvider, useDeleteProvider } from "~/hooks/useProviders";
import type { Provider, CreateProviderData } from "~/api";

type FormState = { name: string; type: string; api_key: string; base_url: string };
const empty: FormState = { name: "", type: "", api_key: "", base_url: "" };

export default function ProvidersPage() {
  const { data: providers, isLoading, error } = useProviders();
  const createProvider = useCreateProvider();
  const updateProvider = useUpdateProvider();
  const deleteProvider = useDeleteProvider();

  const [form, setForm] = useState<FormState>(empty);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  function handleEdit(p: Provider) {
    setEditingId(p.id);
    setForm({ name: p.name, type: p.type, api_key: "", base_url: p.base_url ?? "" });
    setShowForm(true);
  }

  function handleCancel() {
    setForm(empty);
    setEditingId(null);
    setShowForm(false);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const data: CreateProviderData = {
      name: form.name,
      type: form.type,
      api_key: form.api_key,
      base_url: form.base_url || undefined,
    };
    if (editingId) {
      await updateProvider.mutateAsync({ id: editingId, data });
    } else {
      await createProvider.mutateAsync(data);
    }
    handleCancel();
  }

  const isPending = createProvider.isPending || updateProvider.isPending;
  const mutateError = createProvider.error ?? updateProvider.error;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Providers</h1>
        {!showForm && (
          <button
            onClick={() => setShowForm(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm"
          >
            Add Provider
          </button>
        )}
      </div>

      {showForm && (
        <form onSubmit={handleSubmit} className="bg-white border rounded p-4 mb-6 space-y-3">
          <h2 className="font-semibold">{editingId ? "Edit Provider" : "New Provider"}</h2>
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
              <label className="block text-sm font-medium mb-1">Type</label>
              <input
                value={form.type}
                onChange={(e) => setForm({ ...form, type: e.target.value })}
                placeholder="openai / anthropic / ..."
                required
                className="w-full border rounded px-3 py-2 text-sm"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">
                API Key {editingId && <span className="text-gray-400">(leave blank to keep current)</span>}
              </label>
              <input
                type="password"
                value={form.api_key}
                onChange={(e) => setForm({ ...form, api_key: e.target.value })}
                required={!editingId}
                className="w-full border rounded px-3 py-2 text-sm"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Base URL (optional)</label>
              <input
                value={form.base_url}
                onChange={(e) => setForm({ ...form, base_url: e.target.value })}
                placeholder="https://api.openai.com/v1"
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

      {providers && providers.length === 0 && (
        <p className="text-gray-500">No providers yet.</p>
      )}

      {providers && providers.length > 0 && (
        <table className="w-full bg-white border rounded">
          <thead>
            <tr className="border-b bg-gray-50 text-left text-sm text-gray-600">
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Type</th>
              <th className="px-4 py-3">Base URL</th>
              <th className="px-4 py-3"></th>
            </tr>
          </thead>
          <tbody>
            {providers.map((p) => (
              <tr key={p.id} className="border-b last:border-0 hover:bg-gray-50">
                <td className="px-4 py-3 text-sm font-medium">{p.name}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{p.type}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{p.base_url ?? "—"}</td>
                <td className="px-4 py-3 text-right space-x-2">
                  <button onClick={() => handleEdit(p)} className="text-sm text-blue-600 hover:underline">Edit</button>
                  <button
                    onClick={() => deleteProvider.mutate(p.id)}
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
