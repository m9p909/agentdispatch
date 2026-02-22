# Stakeholder Decisions - Telegram Integration Feature

**Date:** 2026-02-15
**Version:** 1.0
**SRS Updated:** TELEGRAM_ACCESS_SRS.md v1.2

## Summary
This document captures all stakeholder decisions made for the Telegram access feature to streamline implementation scope and reduce complexity.

---

## Architecture Decisions

### 1. Message Polling Strategy
**Decision:** Use Telegram polling (getUpdates)
- **Alternative Considered:** Webhook-based updates
- **Rationale:** Simpler implementation, natural fit for Axum async architecture
- **Implementation:** Continuous polling loop with 1-second interval
- **Config Variable:** `TELEGRAM_POLLING_INTERVAL_SECONDS=1` (configurable)

### 2. Bot Token Model
**Decision:** One Telegram bot token per agent (not shared)
- **Alternative Considered:** Single bot token for multiple agents
- **Rationale:** Simpler routing logic, clearer access control, matches existing agents model
- **Impact:** Each agent can have at most one active Telegram bot token
- **Database:** `telegram_configs` table with `UNIQUE(agent_id)` constraint

### 3. Response Format
**Decision:** Support Telegram markdown formatting
- **Alternative Considered:** Plain text only
- **Rationale:** Enhanced user experience without significant complexity
- **Implementation:** Pass `parse_mode: "Markdown"` to Telegram `sendMessage` API
- **Note:** Responses limited to 4096 characters per Telegram API constraint

### 4. Conversation Persistence
**Decision:** No automatic conversation history reset
- **Alternative Considered:** Auto-reset daily/weekly
- **Rationale:** Users expect conversation continuity
- **Administration:** Administrators can manually delete sessions/messages if needed
- **Investigation:** Future work to evaluate cleanup policies and log retention
- **Database:** Sessions and messages persist indefinitely

### 5. Message Editing Support
**Decision:** Not supported - edited messages treated as new messages
- **Alternative Considered:** Handle `edit_message` Telegram updates
- **Rationale:** Simplifies implementation, avoids redo/retry complexity
- **Implementation:** Ignore Telegram `edit_message` updates, no special handling

---

## Performance & Timeout Decisions

### Response Timeout
**Decision:** 2 minutes (120 seconds) hard limit on LLM responses
- **Previous Consideration:** 30 seconds
- **Rationale:** Give LLM providers sufficient time for complex requests
- **Behavior:** Return generic error if LLM doesn't respond within 2 minutes
- **Config Variable:** `TELEGRAM_MESSAGE_TIMEOUT_SECONDS=120` (configurable)

### Polling Interval
**Decision:** 1 second interval for Telegram polling
- **Rationale:** Balance responsiveness vs. API rate limits
- **Config Variable:** `TELEGRAM_POLLING_INTERVAL_SECONDS=1` (configurable)
- **Implementation:** Continuous loop with sleep between iterations

---

## Error Handling & Resilience Decisions

### Retry Strategy
**Decision:** No automatic retries - fail-fast approach
- **Alternative Considered:** Exponential backoff (1s, 2s, 4s)
- **Rationale:** Simpler implementation, internal code handles retry if needed
- **Scope:** No retry logic at Telegram integration layer
- **Error Flow:** Catch error → Log → Return user-friendly message → Exit

### Rate Limiting
**Decision:** Not implemented at application level
- **Alternative Considered:** 10 messages/min per user per agent
- **Rationale:** Simpler implementation, rate limiting enforced by:
  - Telegram API (handles abuse)
  - LLM provider throttling (handles overload)
- **Future:** Add per-user rate limiting if abuse detected in production

---

## Logging & Monitoring Decisions

### Logging Strategy
**Decision:** Console logging only (no database persistence)
- **Alternative Considered:** Persistent `telegram_logs` table
- **Rationale:** Simplifies schema, reduces complexity
- **Implementation:** Use tracing crate (consistent with existing application)
- **Scope:**
  - Message received/sent
  - Whitelist accept/deny
  - Errors (API, LLM, database)
  - Key events for debugging

### Log Output
- **Destination:** Console (stdout/stderr)
- **Format:** Structured logging via tracing crate
- **Retention:** Managed by deployment environment (docker logs, systemd journal, etc.)
- **No Database:** No `telegram_logs` table

### Monitoring & Alerting
**Decision:** Out of scope for initial implementation
- **Alternative Considered:** Dashboard, metrics collection, alerts
- **Rationale:** Console logging provides sufficient visibility
- **Future:** Add metrics dashboard if operational needs emerge

---

## Administrative Features Decisions

### Status Dashboard
**Decision:** Out of scope for initial implementation
- **Alternative Considered:** Real-time agent status page
- **Rationale:** Reduces scope, not critical for MVP
- **Workaround:** View configuration directly via agent management interface

### Export/History Feature
**Decision:** Out of scope for initial implementation
- **Alternative Considered:** CSV/JSON export of interactions
- **Rationale:** Reduces scope, conversations accessible via database
- **Workaround:** Direct database access or console log analysis

### Deployment Runbook
**Decision:** Out of scope
- **Alternative Considered:** Comprehensive deployment guide
- **Rationale:** Standard Rust/Docker deployment practices apply
- **Note:** ENCRYPTION_KEY generation documented in ENCRYPTION_SETUP.md

---

## Testing Strategy

### Test Scope
**Decision:** Unit tests only (no integration or performance tests)
- **In Scope:**
  - Encryption/decryption roundtrip
  - Message parsing and validation
  - Whitelist checking logic
  - Response formatting
  - Error handling
  - Timeout behavior
- **Out of Scope:**
  - Performance testing under load
  - Integration tests with real Telegram API
  - End-to-end deployment testing

### Test Automation
- Framework: Rust test module (`#[cfg(test)]`)
- Mock: Telegram API responses mocked
- Coverage: Unit tests for business logic

---

## Configuration

### Required Environment Variables
```
TELEGRAM_POLLING_INTERVAL_SECONDS=1
TELEGRAM_MESSAGE_TIMEOUT_SECONDS=120
ENCRYPTION_KEY=<64-hex-characters>
```

### Optional Environment Variables
- None specified (all above are required)

### Removed Configuration
- `TELEGRAM_RETRY_MAX_ATTEMPTS` (no retries)
- `TELEGRAM_RATE_LIMIT_MESSAGES_PER_MINUTE` (no rate limiting)
- `TELEGRAM_LOG_RETENTION_DAYS` (no database logs)

---

## Database Schema Simplification

### Tables Created
1. **telegram_configs**
   - Bot token (encrypted)
   - Enable/disable flag
   - Timestamps

2. **telegram_whitelists**
   - Agent ID + Telegram user ID (composite unique)
   - Creation timestamp

### Tables NOT Created
- **telegram_logs** - Replaced by console logging

### Indexes
```sql
-- Essential indexes only
CREATE INDEX idx_telegram_whitelists_agent ON telegram_whitelists(agent_id);
CREATE INDEX idx_telegram_configs_enabled ON telegram_configs(is_enabled) WHERE is_enabled = true;
```

---

## Security Implications

### Simplified Due to These Decisions:
1. **No Log Persistence** → No audit trail in database, reduces data privacy concerns
2. **No Rate Limiting** → Rely on external service limits
3. **Fail-Fast** → Less state to manage, simpler error recovery
4. **Console Logging** → Log management delegated to deployment environment

### Maintained Security:
1. **Token Encryption** → ChaCha20-Poly1305 (unchanged)
2. **Whitelist Checking** → Before any processing (unchanged)
3. **User Notification** → No internal error details exposed (unchanged)

---

## Impact on Non-Functional Requirements

| Requirement | Status | Impact |
|---|---|---|
| Response Time | 2-minute timeout | More lenient than original 15s |
| Throughput | No explicit limit | Managed by external services |
| Rate Limiting | Not implemented | Rely on Telegram & LLM providers |
| Availability | No SLA target | Best effort approach |
| Audit Logging | Console only | No compliance auditing |
| Backup/Recovery | Session/message data only | No log tables to backup |

---

## Future Enhancements

These decisions don't preclude future additions:
- [ ] Add per-user rate limiting if abuse detected
- [ ] Persist logs to database for compliance
- [ ] Add status dashboard for operational visibility
- [ ] Add export functionality for analytics
- [ ] Implement retry logic if reliability issues emerge
- [ ] Add monitoring and alerting
- [ ] Support conversation history cleanup policies

---

## Sign-Off

**Approved By:** Development Team / Stakeholder
**Date:** 2026-02-15
**SRS Version:** 1.2 (updated with all stakeholder decisions)

---

## Related Documents

- TELEGRAM_ACCESS_SRS.md - Full specification (v1.2)
- ENCRYPTION_SETUP.md - Encryption implementation details
- CORRECTIONS_MADE.md - Previous corrections (v1.0 → v1.1)
- SRS_UPDATE_SUMMARY.md - Initial SRS alignment with codebase
