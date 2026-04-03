import { useState } from "react";
import { useModels, useCreateModel, useUpdateModel, useDeleteModel } from "~/hooks/adapters/useModels";
import { useProviders } from "~/hooks/adapters/useProviders";
import type { Model, CreateModelData } from "~/api";

type FormState = { provider_id: string; name: string; model_identifier: string };
const empty: FormState = { provider_id: "", name: "", model_identifier: "" };

export default function ModelsPage() {
  const { data: models, isLoading, error } = useModels();
  const { data: providers } = useProviders();
  const createModel = useCreateModel();
  const updateModel = useUpdateModel();
  const deleteModel = useDeleteModel();

  const [form, setForm] = useState<FormState>(empty);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  function handleEdit(m: Model) {
    setEditingId(m.id);
    setForm({ provider_id: m.provider_id, name: m.name, model_identifier: m.model_identifier });
    setShowForm(true);
  }

  function handleCancel() {
    setForm(empty);
    setEditingId(null);
    setShowForm(false);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const data: CreateModelData = { ...form };
    if (editingId) {
      await updateModel.mutateAsync({ id: editingId, data });
    } else {
      await createModel.mutateAsync(data);
    }
    handleCancel();
  }

  const isPending = createModel.isPending || updateModel.isPending;
  const mutateError = createModel.error ?? updateModel.error;
  const providerName = (id: string) => providers?.find((p) => p.id === id)?.name ?? id.slice(0, 8);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Models</h1>
        {!showForm && (
          <button
            onClick={() => setShowForm(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm"
          >
            Add Model
          </button>
        )}
      </div>

      {showForm && (
        <form onSubmit={handleSubmit} className="bg-white border rounded p-4 mb-6 space-y-3">
          <h2 className="font-semibold">{editingId ? "Edit Model" : "New Model"}</h2>
          {mutateError && <p className="text-red-500 text-sm">{mutateError.message}</p>}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1">Provider</label>
              <select
                value={form.provider_id}
                onChange={(e) => setForm({ ...form, provider_id: e.target.value })}
                required
                className="w-full border rounded px-3 py-2 text-sm"
              >
                <option value="">Select a provider...</option>
                {providers?.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Display Name</label>
              <input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="GPT-4o"
                required
                className="w-full border rounded px-3 py-2 text-sm"
              />
            </div>
            <div className="col-span-2">
              <label className="block text-sm font-medium mb-1">Model Identifier</label>
              <input
                value={form.model_identifier}
                onChange={(e) => setForm({ ...form, model_identifier: e.target.value })}
                placeholder="gpt-4o"
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

      {models && models.length === 0 && <p className="text-gray-500">No models yet.</p>}

      {models && models.length > 0 && (
        <table className="w-full bg-white border rounded">
          <thead>
            <tr className="border-b bg-gray-50 text-left text-sm text-gray-600">
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Identifier</th>
              <th className="px-4 py-3">Provider</th>
              <th className="px-4 py-3"></th>
            </tr>
          </thead>
          <tbody>
            {models.map((m) => (
              <tr key={m.id} className="border-b last:border-0 hover:bg-gray-50">
                <td className="px-4 py-3 text-sm font-medium">{m.name}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{m.model_identifier}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{providerName(m.provider_id)}</td>
                <td className="px-4 py-3 text-right space-x-2">
                  <button onClick={() => handleEdit(m)} className="text-sm text-blue-600 hover:underline">Edit</button>
                  <button
                    onClick={() => deleteModel.mutate(m.id)}
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
