import { useState } from "react";
import { useAgents } from "~/hooks/adapters/useAgents";
import {
  useTelegramConnectors,
  useCreateTelegramConnector,
  useSetTelegramEnabled,
  useDeleteTelegramConnector,
  useTelegramWhitelist,
  useAddWhitelistEntry,
  useRemoveWhitelistEntry,
} from "~/hooks/adapters/useTelegramConnectors";
import type { TelegramConnector } from "~/api";

// ===== Whitelist manager for a single connector =====

function WhitelistManager({ connector }: { connector: TelegramConnector }) {
  const { data: whitelist, isLoading } = useTelegramWhitelist(connector.agent_id);
  const addEntry = useAddWhitelistEntry(connector.agent_id);
  const removeEntry = useRemoveWhitelistEntry(connector.agent_id);
  const [newUserId, setNewUserId] = useState("");

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    const uid = parseInt(newUserId, 10);
    if (isNaN(uid)) return;
    await addEntry.mutateAsync({ telegram_user_id: uid });
    setNewUserId("");
  }

  return (
    <div className="mt-3 pl-4 border-l-2 border-gray-200">
      <p className="text-xs font-medium text-gray-500 mb-2">Whitelist</p>
      {isLoading && <p className="text-xs text-gray-400">Loading...</p>}

      {whitelist && whitelist.length === 0 && (
        <p className="text-xs text-gray-400 mb-2">No users whitelisted — nobody can use this bot.</p>
      )}

      {whitelist && whitelist.length > 0 && (
        <ul className="space-y-1 mb-2">
          {whitelist.map((uid) => (
            <li key={uid} className="flex items-center justify-between text-sm">
              <span className="font-mono text-gray-700">{uid}</span>
              <button
                onClick={() => removeEntry.mutate(uid)}
                className="text-xs text-red-500 hover:underline"
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}

      <form onSubmit={handleAdd} className="flex gap-2">
        <input
          type="number"
          value={newUserId}
          onChange={(e) => setNewUserId(e.target.value)}
          placeholder="Telegram user ID"
          className="border rounded px-2 py-1 text-xs flex-1"
        />
        <button
          type="submit"
          disabled={addEntry.isPending || !newUserId}
          className="bg-blue-600 text-white px-3 py-1 rounded text-xs hover:bg-blue-700 disabled:opacity-50"
        >
          Add
        </button>
      </form>
      {addEntry.error && (
        <p className="text-xs text-red-500 mt-1">{addEntry.error.message}</p>
      )}
    </div>
  );
}

// ===== Main page =====

type FormState = { agent_id: string; bot_token: string };
const emptyForm: FormState = { agent_id: "", bot_token: "" };

export default function ConnectorsPage() {
  const { data: agents } = useAgents();
  const { data: connectors, isLoading, error } = useTelegramConnectors();
  const createConnector = useCreateTelegramConnector();
  const setEnabled = useSetTelegramEnabled();
  const deleteConnector = useDeleteTelegramConnector();

  const [form, setForm] = useState<FormState>(emptyForm);
  const [showForm, setShowForm] = useState(false);
  const [showToken, setShowToken] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Index agents by id for quick lookup
  const agentMap = Object.fromEntries((agents ?? []).map((a) => [a.id, a.name]));

  // Which agents don't already have a connector?
  const usedAgentIds = new Set((connectors ?? []).map((c) => c.agent_id));
  const availableAgents = (agents ?? []).filter((a) => !usedAgentIds.has(a.id));

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    await createConnector.mutateAsync(form);
    setForm(emptyForm);
    setShowForm(false);
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Connectors</h1>
        {!showForm && availableAgents.length > 0 && (
          <button
            onClick={() => setShowForm(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm"
          >
            Add Telegram Connector
          </button>
        )}
      </div>

      {showForm && (
        <form
          onSubmit={handleCreate}
          className="bg-white border rounded p-4 mb-6 space-y-3"
        >
          <h2 className="font-semibold">New Telegram Connector</h2>
          {createConnector.error && (
            <p className="text-red-500 text-sm">{createConnector.error.message}</p>
          )}
          <div>
            <label className="block text-sm font-medium mb-1">Agent</label>
            <select
              value={form.agent_id}
              onChange={(e) => setForm({ ...form, agent_id: e.target.value })}
              required
              className="w-full border rounded px-3 py-2 text-sm"
            >
              <option value="">Select an agent…</option>
              {availableAgents.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Bot Token</label>
            <div className="flex gap-2">
              <input
                type={showToken ? "text" : "password"}
                value={form.bot_token}
                onChange={(e) => setForm({ ...form, bot_token: e.target.value })}
                placeholder="Paste your BotFather token"
                required
                className="flex-1 border rounded px-3 py-2 text-sm font-mono"
              />
              <button
                type="button"
                onClick={() => setShowToken((v) => !v)}
                className="border rounded px-3 py-2 text-sm text-gray-500 hover:text-gray-800 hover:border-gray-400"
              >
                {showToken ? "Hide" : "Show"}
              </button>
            </div>
          </div>
          <div className="flex gap-2">
            <button
              type="submit"
              disabled={createConnector.isPending}
              className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 text-sm disabled:opacity-50"
            >
              {createConnector.isPending ? "Connecting…" : "Create"}
            </button>
            <button
              type="button"
              onClick={() => {
                setForm(emptyForm);
                setShowForm(false);
              }}
              className="text-sm text-gray-500 hover:underline"
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {isLoading && <p className="text-gray-500">Loading…</p>}
      {error && <p className="text-red-500">Error: {error.message}</p>}

      {connectors && connectors.length === 0 && (
        <p className="text-gray-500">No Telegram connectors configured.</p>
      )}

      {connectors && connectors.length > 0 && (
        <div className="space-y-3">
          {connectors.map((c) => (
            <div key={c.id} className="bg-white border rounded p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-sm">
                    {agentMap[c.agent_id] ?? c.agent_id}
                  </p>
                  <p className="text-xs text-gray-500 font-mono mt-0.5">
                    token: {c.masked_token}
                  </p>
                </div>
                <div className="flex items-center gap-3">
                  <label className="flex items-center gap-1.5 cursor-pointer text-sm">
                    <input
                      type="checkbox"
                      checked={c.is_enabled}
                      onChange={(e) =>
                        setEnabled.mutate({
                          agentId: c.agent_id,
                          data: { is_enabled: e.target.checked },
                        })
                      }
                      className="rounded"
                    />
                    {c.is_enabled ? "Enabled" : "Disabled"}
                  </label>
                  <button
                    onClick={() =>
                      setExpandedId(expandedId === c.id ? null : c.id)
                    }
                    className="text-sm text-blue-600 hover:underline"
                  >
                    {expandedId === c.id ? "Hide whitelist" : "Whitelist"}
                  </button>
                  <button
                    onClick={() => deleteConnector.mutate(c.agent_id)}
                    className="text-sm text-red-500 hover:underline"
                  >
                    Delete
                  </button>
                </div>
              </div>

              {expandedId === c.id && <WhitelistManager connector={c} />}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
