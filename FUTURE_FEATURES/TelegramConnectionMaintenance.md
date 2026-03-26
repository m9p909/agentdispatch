# Distributed Telegram Long-Poll Architecture

## Core Problem

Telegram long-polling requires persistent outbound connections, each tied to a unique `offset` (the last update ID seen). As the backend scales horizontally, you need:

- Exactly one poller per connection at a time
- The offset persisted so any instance can resume without missing or duplicating updates
- Connections distributed across instances as you scale

---

## Connection Ownership (The Core Idea)

Each Telegram connection is a named, claimable resource in a shared registry (Redis or Postgres):

```
conn:bot_123 → { owner: instance-A, offset: 94210, lease: t+30s }
conn:bot_456 → { owner: instance-B, offset: 11882, lease: t+30s }
conn:bot_789 → { owner: instance-A, offset: 5001,  lease: t+30s }
```

Each instance:
1. **Claims** unclaimed or expired connections on startup
2. **Heartbeats** its leases every N seconds
3. On crash/eviction, leases expire and other instances absorb the orphaned connections

---

## Offset Handling

Persist the offset **before** processing each batch — not after:

```
1. getUpdates(offset)     ← blocks until update arrives
2. persist new offset     ← write to registry first
3. process the update
4. loop
```

If you crash between steps 2 and 3, you re-process one update. If you crash between 1 and 2, you re-fetch the same update. Both are safe as long as handlers are idempotent.

---

## Rust Implementation

### One Tokio task per connection

```rust
async fn run_poller(
    conn: Connection,
    registry: Arc<Registry>,
    cancel: CancellationToken,
) {
    let _guard = registry.lease_guard(conn.id).await;
    let mut offset = registry.get_offset(conn.id).await;

    loop {
        let updates = tokio::select! {
            result = telegram::get_updates(&conn.token, offset) => result,
            _ = cancel.cancelled() => return,
        };

        registry.save_offset(conn.id, updates.next_offset).await;

        for update in &updates.items {
            handler.handle(update).await;
        }

        offset = updates.next_offset;
    }
}
```

The `_guard` pattern uses `Drop` to release the lease automatically when the task exits for any reason.

### Supervisor loop with JoinSet

```rust
async fn supervisor(registry: Arc<Registry>) {
    let mut set = JoinSet::new();

    loop {
        // blocks until any task finishes for any reason
        set.join_next().await;

        tokio::time::sleep(Duration::from_secs(1)).await;

        // attempt to reclaim and re-spawn
        if let Some(lease) = registry.try_claim(conn.id).await {
            set.spawn(run_poller(conn.clone(), lease));
        }
        // if claim fails, another instance already grabbed it
    }
}
```

---

## Crash Recovery

Two paths work in parallel:

| Path | Mechanism | Speed |
|---|---|---|
| Local supervisor | `JoinSet` detects exit, re-spawns with backoff | Seconds |
| Cross-instance | Redis TTL expires, other instance claims | TTL duration |

### Exponential backoff on repeated crashes

```rust
fn backoff(failures: u32) -> Duration {
    let secs = (2u64.pow(failures)).min(300); // cap at 5 minutes
    Duration::from_secs(secs)
}
```

Prevents hammering Telegram's API during a persistent failure.

---

## Rebalancing

When a new instance comes up, it steals connections from overloaded peers by force-expiring their leases. The previous owner notices on its next heartbeat and drops those pollers. This mirrors Kafka's consumer group rebalancing.

---

## Idempotency

Every update handler must be safe to run twice. Since a crash between persisting the offset and finishing processing can cause a re-delivery, idempotency is the safety net that makes the whole system correct.

---

## Key Crates

| Purpose | Crate |
|---|---|
| Async runtime | `tokio` |
| Cancellation | `tokio-util` (CancellationToken) |
| Redis | `redis` + `bb8` or `deadpool-redis` |
| Telegram client | `teloxide` or `frankenstein` |
| Structured concurrency | `tokio::task::JoinSet` |