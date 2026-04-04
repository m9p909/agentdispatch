# Cancel In-Progress Requests (Web UI + Telegram)

## Context
Users need to stop a running LLM stream mid-response. Two surfaces: web UI (AbortController already exists but isn't exposed) and Telegram (`/cancel` command). Key insight: the Telegram `run_connection` loop is sequential — `/cancel` can't be processed while `stream_to_text` is blocking. Concurrency via per-user worker tasks is required.

---

## Part 1: Web UI Cancel Button

### Step 1 — Add `CANCEL` action to `useMessageManager`
**File:** `frontend/agent-dispatch/app/hooks/manager/useMessageManager.ts`

- Add `{ type: "CANCEL" }` to `Action` union
- Reducer case: `{ ...state, active: false, queue: [], timeline: timeline-without-trailing-tokens-item }`
  - Strip any trailing `tokens` item (partial response) from timeline on cancel
- Add `cancel()`: call `abortRef.current?.abort()` then dispatch `CANCEL`
- Return `cancel` from the hook

### Step 2 — Add Cancel button to chat UI
**File:** `frontend/agent-dispatch/app/routes/chat.tsx`

- Pull `cancel` from hook: `const { timeline, send, isPending, cancel } = useMessageManager(id!)`
- Show Cancel button **alongside** the Send button (Send remains, just disabled while pending)
- Cancel button visible only when `isPending === true`, red/outline style, calls `cancel()`

---

## Part 2: Telegram `/cancel` Command

### Step 3 — Refactor `run_connection` to per-user worker tasks
**File:** `src/telegram/telegram_supervisor.rs`

The current sequential loop blocks on `stream_to_text`. Replace with a dispatcher pattern:

- `run_connection` maintains `HashMap<i64, (mpsc::Sender<String>, CancellationToken)>` keyed by `telegram_user_id`
- On each incoming update, `run_connection` (the dispatch loop) handles it immediately:
  - **`/cancel`**: cancel the token for that user; reply "Request cancelled." or "Nothing to cancel." — no blocking call
  - **Normal message**: look up or spawn a worker task for that user, send message text via the channel
- Each worker task owns: `mpsc::Receiver<String>`, `CancellationToken`, session state
  - Reads messages sequentially (preserves ordering per user)
  - Creates a fresh `CancellationToken` per message, stores it in the shared cancel map
  - Calls `stream_to_text` with the token; on `Ok(None)` (cancelled), sends "Request cancelled." to Telegram

### Step 4 — Thread cancellation token into `stream_to_text` and `run_agent_loop`
**File:** `src/messages/message_service.rs`, `src/messages/agent_loop_service.rs`

- Change `stream_to_text` signature:
  ```rust
  pub async fn stream_to_text(
      &self, session_id: Uuid, content: String,
      cancel: Option<CancellationToken>,
  ) -> Result<Option<String>>
  ```
  Returns `Ok(None)` if cancelled, `Ok(Some(text))` otherwise
- Update the inner `while let Some(event) = stream.next().await` loop to use `tokio::select!` against `cancel.cancelled()` when token is `Some`
- Propagate the token into `run_agent_loop` so the inner chunk stream also honours cancellation:
  - `run_agent_loop` accepts `Option<CancellationToken>`
  - Wraps `while let Some(chunk) = chunk_stream.next().await` with `tokio::select!`
- Existing callers (web SSE route) pass `None` — no behaviour change

### Step 5 — Add `tokio-util` to `Cargo.toml`
**File:** `Cargo.toml`

```toml
tokio-util = "0.7"   # no feature flag needed; CancellationToken is in default build
```

---

## Files to change

| File | Change |
|------|--------|
| `frontend/agent-dispatch/app/hooks/manager/useMessageManager.ts` | CANCEL action, strip partial tokens on cancel, expose cancel() |
| `frontend/agent-dispatch/app/routes/chat.tsx` | Cancel button alongside Send |
| `src/telegram/telegram_supervisor.rs` | Per-user worker + dispatcher, /cancel handling |
| `src/messages/message_service.rs` | `stream_to_text` optional cancel token, returns `Option<String>` |
| `src/messages/agent_loop_service.rs` | Propagate cancel token into chunk stream loop |
| `Cargo.toml` | Add `tokio-util = "0.7"` |

---

## Verification

**Web UI:**
1. Send a long message, click Cancel mid-stream — partial tokens removed from timeline, no error bubble
2. Send button re-enables immediately; can send new message right after
3. Queued messages are discarded after cancel

**Telegram:**
1. Send a message, quickly send `/cancel` — bot replies "Request cancelled." (not after LLM finishes — immediately)
2. `/cancel` with nothing in progress → "Nothing to cancel."
3. Send another message after cancel — bot responds normally
4. Two messages in quick succession — processed in order per user, no race

## Decisions
- `/cancel` goes through normal whitelist check (non-whitelisted users rejected)
- Cancel strips partial tokens from web timeline (no partial message shown)
- Telegram sends "Request cancelled." when a request is actually cancelled; "Nothing to cancel." otherwise
- `/cancel` is case-insensitive (`text.trim().to_lowercase() == "/cancel"`)
- Send button stays visible but disabled while `isPending`; Cancel button appears alongside it
