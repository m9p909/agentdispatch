# Telegram Connector — Object-Oriented Design

## Context

Users connect agents to Telegram so end-users can chat via Telegram DMs. The admin configures a Bot Token (from BotFather) per agent, manages a whitelist of allowed Telegram User IDs, and the system polls Telegram for messages and routes them through the existing LLM + session/message infrastructure.

The polling layer must be horizontally scalable (multiple backend instances) and crash-safe. Each Telegram connection is a claimable resource — exactly one instance owns it at a time via a Postgres-backed lease. Offsets are persisted before processing so any instance can resume without missing or duplicating updates.

---

## Class Design

### Backend (Rust)

```
src/telegram/
├── mod.rs                    — exports
├── db.rs                     — data models + SQL
├── telegram_service.rs       — CRUD business logic (config + whitelist)
├── telegram_adapter.rs       — Telegram Bot API wrapper
├── telegram_registry.rs      — lease + offset ownership (Postgres-backed)
└── telegram_supervisor.rs    — JoinSet supervisor + per-connection polling tasks
```

---

#### TelegramAdapter  *(wraps external Telegram Bot API)*

```rust
pub struct TelegramAdapter {
    client: reqwest::Client,
}

impl TelegramAdapter {
    pub fn new() -> Self
    pub async fn verify_token(&self, token: &str) -> Result<BotInfo>
        // GET /getMe — validates token at config time
    pub async fn get_updates(&self, token: &str, offset: i64) -> Result<Vec<TelegramUpdate>>
        // GET /getUpdates?offset=N — long-polls for new messages
    pub async fn send_message(&self, token: &str, chat_id: i64, text: &str) -> Result<()>
        // POST /sendMessage — sends LLM response back to user
}
```

**DTOs (serde Deserialize from Telegram API JSON):**
- `BotInfo { id: i64, username: String }`
- `TelegramUpdate { update_id: i64, message: Option<TelegramMessage> }`
- `TelegramMessage { message_id: i64, from: TelegramUser, chat: TelegramChat, text: Option<String> }`
- `TelegramUser { id: i64, is_bot: bool, first_name: String }`
- `TelegramChat { id: i64 }`

---

#### TelegramConnectorService  *(CRUD for connector config + whitelist)*

```rust
pub struct TelegramConnectorService {
    db: Database,
    crypto: CryptoService,
    telegram: TelegramAdapter,
}

impl TelegramConnectorService {
    pub fn new(db: Database, crypto: CryptoService, telegram: TelegramAdapter) -> Self

    // Connector CRUD
    pub async fn create_connector(&self, agent_id: Uuid, bot_token: &str) -> Result<ConnectorResponse>
        // 1. verify_token() — 400 if invalid
        // 2. Encrypt token
        // 3. Insert into telegram_configs
    pub async fn get_connector(&self, agent_id: Uuid) -> Result<Option<ConnectorResponse>>
    pub async fn list_connectors(&self) -> Result<Vec<ConnectorResponse>>
    pub async fn set_enabled(&self, agent_id: Uuid, enabled: bool) -> Result<ConnectorResponse>
    pub async fn delete_connector(&self, agent_id: Uuid) -> Result<()>

    // Whitelist management
    pub async fn add_whitelist_entry(&self, agent_id: Uuid, telegram_user_id: i64) -> Result<()>
    pub async fn remove_whitelist_entry(&self, agent_id: Uuid, telegram_user_id: i64) -> Result<()>
    pub async fn get_whitelist(&self, agent_id: Uuid) -> Result<Vec<i64>>
}
```

**`ConnectorResponse`** (never exposes raw token):
```rust
pub struct ConnectorResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub is_enabled: bool,
    pub masked_token: String,   // e.g. "...ab1c"
    pub created_at: DateTime<Utc>,
}
```

---

#### TelegramRegistry  *(Postgres-backed lease + offset ownership)*

Owns the distributed coordination state. Short, committed transactions only — no long holds.

```rust
pub struct TelegramRegistry {
    db: Database,
    instance_id: String,   // unique per process (UUID generated at startup)
}

impl TelegramRegistry {
    pub fn new(db: Database) -> Self   // generates instance_id = Uuid::new_v4()

    // Returns connectors this instance can claim (lease expired or unclaimed)
    pub async fn list_claimable(&self) -> Result<Vec<TelegramConfig>>
        // SELECT WHERE is_enabled AND (lease_expires_at IS NULL OR lease_expires_at < now())

    // Atomically claims one connector; returns false if another instance beat us to it
    pub async fn try_claim(&self, connector_id: Uuid) -> Result<bool>
        // UPDATE telegram_configs
        //   SET owner_instance_id = self.instance_id,
        //       lease_expires_at = now() + interval '30 seconds'
        //   WHERE id = connector_id
        //     AND (lease_expires_at IS NULL OR lease_expires_at < now())

    // Extends the lease — called every ~10s by each active poller task
    pub async fn heartbeat(&self, connector_id: Uuid) -> Result<()>

    // Persists the offset — called BEFORE processing each batch (crash-safe)
    pub async fn save_offset(&self, connector_id: Uuid, offset: i64) -> Result<()>

    // Releases the lease — called on clean shutdown
    pub async fn release(&self, connector_id: Uuid) -> Result<()>

    // Returns the persisted offset for a connector (used on claim/resume)
    pub async fn get_offset(&self, connector_id: Uuid) -> Result<i64>
}
```

---

#### TelegramSupervisor  *(JoinSet supervisor + per-connection poller tasks)*

```rust
pub struct TelegramSupervisor {
    registry: Arc<TelegramRegistry>,
    telegram: TelegramAdapter,
    connector_service: TelegramConnectorService,
    session_service: SessionService,
    message_service: MessageService,
}

impl TelegramSupervisor {
    pub fn new(...) -> Self
    pub fn start(self) -> tokio::task::JoinHandle<()>
        // Spawns a tokio task running supervisor_loop()

    async fn supervisor_loop(&self)
        // Every 1s:
        //   1. list_claimable() from registry
        //   2. try_claim() each; on success, spawn run_connection() into JoinSet
        //   3. join_next() to reap finished/crashed tasks (non-blocking)
        //   4. Re-claim crashed connectors on next cycle (their lease will have expired)

    async fn run_connection(
        config: TelegramConfig,
        registry: Arc<TelegramRegistry>,
        telegram: TelegramAdapter,
        cancel: CancellationToken,
        // + session/message services
    )
        // 1. Load offset from registry
        // 2. Loop:
        //    a. tokio::select! { get_updates() | cancel.cancelled() }
        //    b. registry.save_offset() ← BEFORE processing
        //    c. For each update: process_update()
        //    d. registry.heartbeat() every ~10s
        // 3. On cancel or error: registry.release(), apply backoff before exit

    async fn process_update(update: TelegramUpdate, config: &TelegramConfig, ...) -> Result<()>
        // 1. Skip non-text messages
        // 2. Check whitelist — denied: send rejection, log
        // 3. Find or create session for (agent_id, telegram_user_id)
        // 4. Idempotency check: skip if message_id already in session messages
        // 5. message_service.create_message(session_id, text) → stores + calls LLM
        // 6. telegram.send_message(chat_id, ai_response)
        // 7. Log outcome
}
```

**Backoff on repeated crashes:**
```rust
fn backoff(failures: u32) -> Duration {
    Duration::from_secs((2u64.pow(failures)).min(300))  // caps at 5 minutes
}
```

---

#### DB Models  *(src/telegram/db.rs)*

```rust
pub struct TelegramConfig {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub bot_token: String,                      // encrypted at rest
    pub is_enabled: bool,
    pub last_update_id: i64,                    // persisted polling offset
    pub owner_instance_id: Option<String>,      // which backend instance owns this
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct TelegramWhitelistEntry {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub telegram_user_id: i64,
    pub created_at: DateTime<Utc>,
}
```

---

#### Schema  *(src/schema.rs)*

```sql
CREATE TABLE telegram_configs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id UUID NOT NULL UNIQUE REFERENCES agents(id) ON DELETE CASCADE,
  bot_token TEXT NOT NULL,                    -- encrypted (ChaCha20-Poly1305)
  is_enabled BOOLEAN NOT NULL DEFAULT true,
  last_update_id BIGINT NOT NULL DEFAULT 0,  -- polling offset, persisted before processing
  owner_instance_id TEXT,                    -- NULL = unclaimed
  lease_expires_at TIMESTAMPTZ,              -- NULL = unclaimed; expires = claimable
  created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_telegram_configs_claimable
  ON telegram_configs(is_enabled, lease_expires_at)
  WHERE is_enabled = true;

CREATE TABLE telegram_whitelists (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  telegram_user_id BIGINT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(agent_id, telegram_user_id)
);
```

---

#### API Routes  *(src/routes.rs)*

```
GET    /api/v1/connectors/telegram                          — list connectors
POST   /api/v1/connectors/telegram                          — create { agent_id, bot_token }
GET    /api/v1/connectors/telegram/:agent_id                — get one
PATCH  /api/v1/connectors/telegram/:agent_id                — toggle { is_enabled }
DELETE /api/v1/connectors/telegram/:agent_id                — remove

GET    /api/v1/connectors/telegram/:agent_id/whitelist      — list whitelist
POST   /api/v1/connectors/telegram/:agent_id/whitelist      — add { telegram_user_id }
DELETE /api/v1/connectors/telegram/:agent_id/whitelist/:uid — remove entry
```

`TelegramConnectorService` added to `AppState`. `TelegramSupervisor` spawned as a background task in `main()` — not in AppState.

---

### Frontend (React/TypeScript)

New route `/connectors` with:
- `app/routes/connectors.tsx` — agent dropdown + token input form, connector list with enabled toggle, inline whitelist manager
- `app/hooks/useTelegramConnectors.ts` — React Query hooks
- `api.ts` additions: `TelegramConnector` interface + `api.telegram.{list, create, setEnabled, delete, whitelist}`

---

## Key Design Decisions

| Decision | Choice | Reason |
|---|---|---|
| Registry backend | Postgres (not Redis) | No new infrastructure; existing DB is sufficient |
| Lease duration | 30s, heartbeat every 10s | Balances failover speed vs. heartbeat overhead |
| Offset timing | Saved BEFORE processing | Safe re-delivery on crash; handlers must be idempotent |
| Idempotency | Check Telegram `message_id` before inserting message | Prevents duplicate LLM calls on re-delivery |
| Task model | One tokio task per connection, supervised by JoinSet | Clean isolation; supervisor detects crashes via join |
| Backoff | Exponential, capped at 5 min | Prevents hammering Telegram on persistent failure |

---

## Crash Recovery Paths

| Scenario | Recovery |
|---|---|
| Poller task panics | `JoinSet` detects exit; supervisor re-claims and re-spawns after backoff |
| Instance process dies | Lease expires after 30s; another instance's supervisor claims the connector |
| DB blip during heartbeat | Lease may expire; another instance claims and resumes from persisted offset |
| Duplicate update on resume | Idempotency check on `message_id` prevents double-processing |

---

## Files to Create/Modify

**New:**
- `src/telegram/{mod,db,telegram_service,telegram_adapter,telegram_registry,telegram_supervisor}.rs`
- `frontend/.../hooks/useTelegramConnectors.ts`
- `frontend/.../routes/connectors.tsx`

**Modify:**
- `src/schema.rs` — add `create_telegram_tables()`
- `src/routes.rs` — add connector routes + `telegram_connectors` to `AppState`
- `src/main.rs` — init services, spawn `TelegramSupervisor`
- `src/lib.rs` — export `telegram` module
- `Cargo.toml` — add `tokio-util` (CancellationToken)
- `frontend/.../api.ts`, `routes.ts`

---

## Verification

1. `cargo build` + `cargo clippy` — clean
2. `cargo run` — schema applied, supervisor starts, logs "no connectors to claim"
3. POST a valid bot token → connector claimed by supervisor within 1s, polling begins
4. POST invalid token → 400
5. Send Telegram message from whitelisted user → LLM response received in Telegram
6. Send from non-whitelisted user → rejection message, no LLM call
7. Kill and restart server → supervisor resumes from persisted `last_update_id`, no duplicate messages
8. Run two instances → each claims different connectors; kill one → other absorbs its connectors within 30s
