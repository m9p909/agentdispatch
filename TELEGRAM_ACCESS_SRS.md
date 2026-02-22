# Software Requirements Specification (SRS)
## Telegram Access Feature for Agent Builder

**Document Version:** 1.0
**Date:** 2026-02-15
**Status:** Draft
**Prepared for:** Agent Builder Development Team

---

## 1. Introduction

### 1.1 Purpose
This Software Requirements Specification defines the functional and non-functional requirements for implementing Telegram access to autonomous agents within the Agent Builder system. This document serves as the contractual agreement between stakeholders and the development team regarding what the Telegram integration feature shall accomplish.

**Intended Audience:**
- Development team (Rust/backend engineers)
- System operators and administrators
- Product stakeholders

### 1.2 Scope

#### 1.2.1 What Will Be Done
- Integrate Telegram Bot API with the Agent Builder system
- Enable agents to receive and respond to messages via Telegram
- Implement user access control via Telegram user ID whitelist
- Store and manage Telegram configuration and message history
- Support text message exchange between Telegram users and agents

#### 1.2.2 What Will NOT Be Done (Out of Scope)
- Telegram media file handling (images, videos, documents, audio, etc.)
- Telegram group chat support
- Message encryption beyond Telegram's native security
- Custom Telegram inline keyboards or buttons
- Multi-language message translation
- Agent-initiated Telegram notifications (pull-based only)
- Webhook-based polling or long-polling optimization

#### 1.2.3 Benefits and Objectives
- **Accessibility:** Enable agents to be accessed through a ubiquitous messaging platform
- **Scalability:** Support multiple agents with individual Telegram integrations
- **Security:** Enforce user access control to prevent unauthorized agent interactions
- **Operational Simplicity:** Provide intuitive text-based agent interaction without UI dependency

### 1.3 Definitions, Acronyms, and Abbreviations

| Term | Definition |
|------|-----------|
| **Agent** | An AI-powered autonomous entity configured in Agent Builder with a system prompt and associated LLM model |
| **Telegram Bot Token** | Unique authentication credential provided by Telegram BotFather for API access |
| **Telegram User ID** | Unique numeric identifier for a Telegram user |
| **User Whitelist** | List of approved Telegram user IDs authorized to interact with a specific agent |
| **Session** | A conversation context between a user and an agent, storing message history |
| **Message** | A discrete communication unit (user question or agent response) within a session |
| **LLM** | Large Language Model (e.g., GPT-4, Claude) used for agent responses |
| **API** | Application Programming Interface |
| **CRUD** | Create, Read, Update, Delete operations |
| **PostgreSQL** | Relational database management system |
| **Axum** | Rust web application framework (v0.7) with async request handling |
| **OpenAI-Compatible** | API specification compatible with OpenAI's chat completion interface |
| **ChaCha20-Poly1305** | AEAD (Authenticated Encryption with Associated Data) encryption algorithm used for storing sensitive credentials |
| **Nonce** | Random 96-bit (12-byte) value used once per encryption operation |
| **ENCRYPTION_KEY** | 256-bit (32-byte) encryption key stored as environment variable for credential encryption |

### 1.4 References

| Reference | Title | Version |
|-----------|-------|---------|
| RFC-001 | Telegram Bot API Documentation | Latest |
| RFC-002 | Agent Builder Architecture | Current |
| RFC-003 | PostgreSQL Schema Specification | Current |
| STD-001 | OpenAI API Specification | v1 |

### 1.5 Document Overview

- **Section 1:** Introduction and scope definition
- **Section 2:** System context, user classes, and operating environment
- **Section 3:** Functional requirements organized by feature
- **Section 4:** External interface specifications (APIs, data formats, protocols)
- **Section 5:** Non-functional requirements (performance, security, reliability)
- **Section 6:** Other requirements (database, deployment, compliance)
- **Section 7:** Appendices with data models, diagrams, and traceability matrix

---

## 2. Overall Description

### 2.1 Product Perspective

The Telegram Access feature extends the existing Agent Builder web application, which is a Rust-based system for creating and managing AI agents. The new feature connects agents to Telegram, enabling users to interact with agents through Telegram Direct Messages.

#### 2.1.1 System Context Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    External Systems                         │
│                                                              │
│  ┌──────────────────┐              ┌──────────────────┐    │
│  │  Telegram Bot    │◄────────────►│   LLM Providers  │    │
│  │      API         │              │  (OpenAI, etc.)  │    │
│  └──────────────────┘              └──────────────────┘    │
│         ▲                                   ▲               │
│         │                                   │               │
│         │                                   │               │
│         │         ┌──────────────────────┐  │               │
│         └────────►│   Agent Builder      │──┘               │
│                   │   Web Application    │                  │
│         ┌────────►│   (Rust/Axum)        │◄────┐           │
│         │         └──────────────────────┘      │           │
│         │                   │                   │           │
│         │                   ▼                   │           │
│         │         ┌──────────────────────┐      │           │
│         │         │    PostgreSQL        │      │           │
│         │         │    Database          │      │           │
│         │         └──────────────────────┘      │           │
│         │                                       │           │
│  ┌──────┴───────────┐              ┌───────────┴──────┐   │
│  │  Telegram Users  │              │   Web Browser    │   │
│  │  (Direct Messages)               │   (Administrators)   │
│  └──────────────────┘              └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### 2.1.2 Data Flow Diagram - Telegram Message Flow

```
Telegram User                 Agent Builder              LLM Provider
     │                              │                         │
     │──(1) Send Message───────────►│                         │
     │                              │                         │
     │                         (2) Save message                │
     │                         (3) Validate user               │
     │                         (4) Load session                │
     │                              │                         │
     │                              │────(5) Get response─────►│
     │                              │                         │
     │                              │◄────(6) Response────────│
     │                              │                         │
     │                         (7) Save response               │
     │                              │                         │
     │◄─(8) Send via Telegram API──│                         │
     │                              │                         │
```

### 2.2 Product Functions

The Telegram Access feature shall provide:

1. **Telegram Bot Configuration Management**
   - Store and manage Telegram bot tokens per agent
   - Verify bot token validity at configuration time
   - Support agent enable/disable for Telegram access

2. **Message Reception and Processing**
   - Receive Telegram messages via Telegram Bot API
   - Parse and validate incoming messages
   - Route messages to appropriate agents based on Telegram configuration

3. **User Access Control**
   - Maintain Telegram user ID whitelists per agent
   - Validate message sender against whitelist before processing
   - Log access attempts (authorized and denied)

4. **Message Response Generation**
   - Process user messages through agent's LLM
   - Generate contextual responses using conversation history
   - Send responses back to user via Telegram

5. **Session and History Management**
   - Create and maintain separate sessions for each Telegram user-agent pair
   - Preserve full conversation history in PostgreSQL
   - Enable administrators to view Telegram interaction history

6. **Error Handling and Recovery**
   - Handle Telegram API failures gracefully
   - Notify users of failures without exposing system internals
   - Log errors for operator investigation

### 2.3 User Classes and Characteristics

#### 2.3.1 End Users (Telegram Message Senders)
- **Characteristics:** Non-technical, using standard Telegram client
- **Expertise Level:** Low technical knowledge
- **Interaction:** Send text messages to agent via Telegram Direct Message
- **Expected Volume:** Variable, depends on agent use case

#### 2.3.2 Administrators (Agent Configuration)
- **Characteristics:** Technical staff managing agents and access control
- **Expertise Level:** Moderate-to-high technical knowledge
- **Interaction:** Configure Telegram bot tokens, manage user whitelists, view interaction logs
- **Expected Volume:** Infrequent configuration, periodic audits

#### 2.3.3 System Operators
- **Characteristics:** Technical staff monitoring system health
- **Expertise Level:** High technical knowledge
- **Interaction:** Monitor logs, troubleshoot Telegram integration issues, manage deployments
- **Expected Volume:** Continuous monitoring, reactive issue resolution

### 2.4 Operating Environment

#### 2.4.1 Hardware Platform
- Server environment capable of running Rust binaries
- Minimum: 1 CPU core, 512MB RAM (baseline)
- Recommended: 2+ CPU cores, 2GB+ RAM (production)
- Storage: PostgreSQL database with sufficient disk space for message history

#### 2.4.2 Software Components
- **Language:** Rust
- **Web Framework:** Axum
- **Database:** PostgreSQL (existing)
- **Telegram Integration:** Telegram Bot API (HTTP-based)
- **External APIs:** OpenAI-compatible LLM endpoints

#### 2.4.3 Operating System
- Linux (primary production target)
- macOS (development support)
- Windows (development support via WSL2)

#### 2.4.4 Network Requirements
- HTTPS outbound connectivity to Telegram Bot API (`api.telegram.org`)
- HTTPS outbound connectivity to configured LLM provider endpoints
- HTTP inbound capability for Telegram updates (webhook or polling-based)

### 2.5 Design Constraints

#### 2.5.1 Regulatory and Standards Compliance
- **Telegram Terms of Service:** Comply with Telegram Bot Platform terms
- **Data Privacy:** Comply with applicable data protection regulations (GDPR where applicable)
- **Rate Limiting:** Respect Telegram API rate limits (enforced by Telegram)

#### 2.5.2 Technical Constraints
- **Message Length:** Telegram imposes 4096-character limit per message
- **Response Time:** System shall not retry indefinitely; fail-fast on LLM timeouts
- **API Compatibility:** LLM providers must support OpenAI-compatible chat completion interface
- **Database Capacity:** PostgreSQL schema must efficiently store message history at scale

#### 2.5.3 Architectural Constraints
- **Single Web Application:** Telegram integration must be built within existing Axum application
- **Existing Database:** Must use existing PostgreSQL schema; no new database dependencies
- **Session Continuity:** Telegram integration reuses existing session/message model

### 2.6 Assumptions and Dependencies

#### 2.6.1 Assumptions
- Telegram bot tokens are valid and will be provided by administrators
- Telegram API remains available with current specification
- PostgreSQL database is properly configured and backed up
- LLM providers remain accessible and responsive
- Administrators will properly maintain user whitelists

#### 2.6.2 Dependencies
- **External:** Telegram Bot API availability and stability
- **External:** LLM provider API availability (OpenAI, etc.)
- **Internal:** Existing Agent Builder database schema (Users, Agents, Sessions, Messages)
- **Internal:** Existing LLM provider and model management system
- **Operational:** Administrator availability for configuration and access control management

---

## 3. Specific Requirements (Functional)

### 3.1 Telegram Agent Configuration

#### FR-001: Add Telegram Configuration to Agent
**Description:** The system shall allow administrators to add a Telegram bot token to any existing agent.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] Administrator can input Telegram bot token via web UI
- [ ] System validates token format (alphanumeric, colon separator)
- [ ] System verifies token by calling Telegram getMe API
- [ ] Valid token is encrypted and stored in database
- [ ] Error message displayed if token is invalid
- [ ] Each agent can have at most one active Telegram token

**Related Data:** Agents table, Telegram configuration record

#### FR-002: Remove Telegram Configuration from Agent
**Description:** The system shall allow administrators to remove Telegram access from any agent.

**Priority:** High
**Acceptance Criteria:**
- [ ] Administrator can disable/remove Telegram token from agent
- [ ] Disabled agent does not respond to Telegram messages
- [ ] Configuration record is deleted or marked inactive
- [ ] Confirmation required before removal
- [ ] Audit log entry created

#### FR-003: View Telegram Configuration
**Description:** The system shall display current Telegram configuration for each agent.

**Priority:** High
**Acceptance Criteria:**
- [ ] Administrator can view enabled/disabled status
- [ ] Bot token is masked (show only last 4 characters)
- [ ] Configuration page shows associated agent name
- [ ] Configuration status is updated in real-time

### 3.2 User Access Control

#### FR-004: Maintain Telegram User Whitelist
**Description:** The system shall maintain a whitelist of approved Telegram user IDs per agent.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] Administrator can add Telegram user IDs to whitelist
- [ ] Administrator can remove Telegram user IDs from whitelist
- [ ] Administrator can view current whitelist
- [ ] Whitelist is agent-specific (different agents have different whitelists)
- [ ] Telegram user IDs are validated as numeric values
- [ ] Changes take effect immediately

**Related Data:** New table: `telegram_whitelist` with (agent_id, telegram_user_id)

#### FR-005: Validate User Authorization
**Description:** The system shall verify that incoming Telegram messages are from whitelisted users before processing.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] Message is checked against whitelist before processing
- [ ] Non-whitelisted users receive rejection message
- [ ] Rejection does not trigger agent response or LLM call
- [ ] Unauthorized access attempt is logged
- [ ] Empty whitelist means agent rejects all Telegram messages

#### FR-006: Log Access Attempts
**Description:** The system shall log all Telegram message attempts (authorized and denied) to console.

**Priority:** High
**Acceptance Criteria:**
- [ ] All Telegram messages logged with: sender ID, agent ID, timestamp, status (accepted/denied), reason if denied
- [ ] Logs output to console (structured logging)
- [ ] No database-based log retention (console logs managed by deployment environment)
- [ ] Logs include sufficient detail for debugging and monitoring

### 3.3 Message Reception and Routing

#### FR-007: Receive Telegram Messages
**Description:** The system shall receive messages sent to the agent's Telegram bot via Telegram Bot API.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] System implements Telegram polling or webhook endpoint (poll-based assumed initially)
- [ ] Messages are retrieved with user ID, chat ID, and text content
- [ ] Message reception is resilient to temporary API failures
- [ ] Failed message retrievals are logged
- [ ] Duplicate messages are detected and not re-processed

#### FR-008: Parse and Validate Incoming Messages
**Description:** The system shall parse Telegram messages and validate content before processing.

**Priority:** High
**Acceptance Criteria:**
- [ ] Message text is extracted and trimmed
- [ ] Empty messages are rejected
- [ ] Messages exceeding 4000 characters are truncated with warning
- [ ] Non-text messages (media, files) are rejected with user notification
- [ ] Validated messages are forwarded to message processor

#### FR-009: Route Messages to Correct Agent
**Description:** The system shall route incoming Telegram messages to the appropriate agent based on bot token.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] Telegram API provides bot token context
- [ ] System maps token to specific agent
- [ ] Message is routed to correct agent-user session
- [ ] Routing errors are logged with bot token and message details

### 3.4 Response Generation

#### FR-010: Generate Agent Response
**Description:** The system shall generate agent responses to Telegram messages using the configured LLM.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] System retrieves agent's system prompt and LLM model
- [ ] Conversation history is loaded from database (reuses existing session logic)
- [ ] LLM is called with: system prompt, full message history, new user message
- [ ] Response is generated using existing LLM integration
- [ ] Response time does not exceed 30 seconds (configurable timeout)
- [ ] LLM timeout results in user-facing error message

#### FR-011: Send Response via Telegram
**Description:** The system shall send generated response to user via Telegram Bot API.

**Priority:** Critical
**Acceptance Criteria:**
- [ ] Response is sent to correct Telegram user ID
- [ ] Response text is properly formatted (no raw JSON, no system prompts exposed)
- [ ] Multi-part responses are sent as sequential messages if exceeding 4096 characters
- [ ] Failed sends are retried up to 3 times
- [ ] Permanent send failures are logged with error details
- [ ] User receives notification of failure if unable to send

#### FR-012: Maintain Conversation Context
**Description:** The system shall preserve full conversation history for each Telegram user-agent pair.

**Priority:** High
**Acceptance Criteria:**
- [ ] Each Telegram user gets unique session per agent
- [ ] Session persists across multiple messages
- [ ] User can refer back to previous messages in context
- [ ] Session history is stored in existing sessions/messages tables
- [ ] Administrator can view full conversation history

### 3.5 Error Handling

#### FR-013: Handle Telegram API Failures
**Description:** The system shall gracefully handle Telegram API errors without retrying.

**Priority:** High
**Acceptance Criteria:**
- [ ] Telegram API HTTP errors (4xx, 5xx) are caught
- [ ] Network timeouts are detected and handled
- [ ] User receives appropriate error message (e.g., "Service temporarily unavailable")
- [ ] System does not expose internal error details to users
- [ ] All failures are logged to console
- [ ] No automatic retry logic (fail-fast approach)

#### FR-014: Handle LLM Provider Failures
**Description:** The system shall gracefully handle LLM provider unavailability with 2-minute timeout.

**Priority:** High
**Acceptance Criteria:**
- [ ] LLM provider timeouts/errors are caught
- [ ] Hard timeout of 2 minutes on LLM API calls
- [ ] User receives user-friendly error message if timeout exceeded
- [ ] No retry logic (fail-fast approach, internal code handles retry)
- [ ] Error is logged to console with LLM provider details
- [ ] Failed responses not sent back (user notified of error only)

#### FR-015: Handle Database Failures
**Description:** The system shall handle database connection and transaction failures.

**Priority:** High
**Acceptance Criteria:**
- [ ] Database connection errors are caught
- [ ] User receives generic "temporary unavailable" message
- [ ] Error is logged with transaction details
- [ ] System does not leave orphaned sessions/messages

### 3.6 Administration and Monitoring

#### FR-016: View Telegram Agent Status
**Description:** Out of scope for initial implementation. Administrators can view agent configuration directly.

**Priority:** Low (future enhancement)
**Note:** Telegram configuration and whitelist viewable via existing agent management interface.

#### FR-017: Export Telegram Interaction History
**Description:** Out of scope for initial implementation. Sessions and messages can be accessed directly from database or console logs.

**Priority:** Low (future enhancement)
**Note:** Conversation history persists in sessions and messages tables for direct access.

---

## 4. External Interface Requirements

### 4.1 User Interfaces

#### 4.1.1 Telegram Bot Interface (End Users)
- **Type:** Telegram Direct Message (DM)
- **Interaction Pattern:** Conversational text exchange
- **Accessibility:** Standard Telegram mobile and desktop clients
- **User Actions:**
  - Send text message to agent bot
  - Receive text response
  - Continue multi-turn conversation

#### 4.1.2 Telegram Management Interface (Administrators)
- **Type:** Web-based admin panel
- **Interaction Pattern:** Forms for configuration and CRUD operations
- **Components:**
  - Agent list with Telegram status indicator
  - Telegram configuration form (token input, validation display)
  - Whitelist management table (add/remove user IDs)
  - Interaction history viewer (filterable table)
  - Export dialog (date range, format selection)

### 4.2 Hardware Interfaces

#### 4.2.1 Telegram Device Support
- **Mobile:** iOS, Android via Telegram clients
- **Desktop:** macOS, Windows, Linux via Telegram clients
- **Web:** Browser-based Telegram Web App (not required for this feature)

### 4.3 Software Interfaces

#### 4.3.1 Telegram Bot API
**Protocol:** HTTPS (REST)
**Base URL:** `https://api.telegram.org/bot{token}`
**Authentication:** Token-based (provided by Telegram BotFather)

**Endpoints Used:**
- `POST /getMe` - Verify bot token validity
- `GET /getUpdates` - Poll for incoming messages (polling approach)
- `POST /sendMessage` - Send responses to users

**Data Format:**
```json
// Incoming message (from getUpdates)
{
  "update_id": 123456789,
  "message": {
    "message_id": 1,
    "from": {
      "id": 987654321,
      "is_bot": false,
      "first_name": "John"
    },
    "chat": {
      "id": 987654321,
      "type": "private"
    },
    "date": 1707964800,
    "text": "Hello agent"
  }
}

// Outgoing message (sendMessage)
{
  "chat_id": 987654321,
  "text": "Response from agent"
}
```

**Error Handling:**
- HTTP 200: Success
- HTTP 400: Bad request (invalid parameters)
- HTTP 401: Unauthorized (invalid token)
- HTTP 429: Rate limited (back off)
- HTTP 500: Server error (retry with exponential backoff)

#### 4.3.2 PostgreSQL Database Interface
**Existing Schema Extensions:**

New Tables (using same patterns as existing `llm_providers` table):
```sql
CREATE TABLE telegram_configs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id UUID NOT NULL UNIQUE REFERENCES agents(id) ON DELETE CASCADE,
  bot_token TEXT NOT NULL,
  is_enabled BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE telegram_whitelists (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  telegram_user_id BIGINT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(agent_id, telegram_user_id)
);

CREATE INDEX IF NOT EXISTS idx_telegram_whitelists_agent
  ON telegram_whitelists(agent_id);

CREATE INDEX IF NOT EXISTS idx_telegram_configs_enabled
  ON telegram_configs(is_enabled) WHERE is_enabled = true;
```

**Encryption Details:**
- `bot_token` field stores hex-encoded ciphertext (nonce + encrypted token)
- Encryption/decryption handled by application layer using `Cipher` struct from `src/crypto.rs`
- Pattern matches existing `llm_providers.api_key` encryption implementation
- Requires `ENCRYPTION_KEY` environment variable (256-bit hex string)

**Logging Details:**
- Interactions logged to console via tracing crate (not persisted to database)
- No `telegram_logs` table (simplifies schema and reduces complexity)

Schema Reuse:
- `agents` table: Links to Telegram config via agent_id (existing, unchanged)
- `sessions` table: One session per (agent, telegram_user), leverages existing session/message model
- `messages` table: Stores Telegram conversation history with ON DELETE CASCADE (existing, unchanged)
- `llm_providers` table: Existing pattern for encrypted secrets storage to be replicated

#### 4.3.3 OpenAI-Compatible LLM API
**Protocol:** HTTPS (REST)
**Authentication:** API key (stored in LLM provider config)
**Endpoint:** `/v1/chat/completions`

**Request Format:**
```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "agent_system_prompt"},
    {"role": "user", "content": "previous user message"},
    {"role": "assistant", "content": "previous agent response"},
    {"role": "user", "content": "current user message"}
  ],
  "temperature": 0.7,
  "max_tokens": 2000
}
```

**Response Format:**
```json
{
  "choices": [
    {"message": {"content": "Generated response text"}}
  ]
}
```

**Integration:** Uses existing LLM service (no changes required)

### 4.4 Communication Interfaces

#### 4.4.1 Message Format and Protocol
- **Protocol:** HTTPS/TLS (Telegram-to-Agent, Agent-to-LLM)
- **Content-Type:** `application/json`
- **Character Encoding:** UTF-8
- **Message Size Limits:**
  - Telegram: 4096 characters per message
  - LLM requests: Configurable (typically 2000-4000 tokens context)
  - Database storage: UNLIMITED (TEXT type)

#### 4.4.2 Reliability and Error Strategy
- **Connection Failures:** Fail-fast, no retries (internal code handles retry if needed)
- **Telegram API Rate Limits:** Respect 429 responses, fail with error to user
- **LLM Timeouts:** Hard limit 2 minutes (120 seconds), no retries (fail-fast)
- **Idempotency:** Duplicate message detection using Telegram message_id
- **Error Messages:** Generic, user-friendly (no internal details exposed)

#### 4.4.3 Security
- **Transport:** All communication via HTTPS/TLS
- **Authentication:** Telegram token-based, LLM API key-based
- **Secrets Storage:** Bot tokens and API keys encrypted at rest in PostgreSQL
- **Access Control:** Whitelist enforced before message processing
- **Audit Logging:** All interactions logged immutably

---

## 5. Non-Functional Requirements

### 5.1 Performance Requirements

#### NFR-001: Message Response Time
**Requirement:** Response timeout shall be 2 minutes (120 seconds). System shall return error if LLM response not received within this time.

**Rationale:** Balance between giving LLM providers sufficient time and preventing indefinite hangs.
**Measurement:** Track from LLM API call to response receipt or timeout.

#### NFR-002: Message Throughput
**Requirement:** System shall support minimum 100 messages per minute across all Telegram agents.

**Rationale:** Support multiple concurrent users and agents.
**Measurement:** Messages processed successfully without queueing or dropping (no explicit rate limiting enforced by system).

#### NFR-003: Database Query Performance
**Requirement:** Session/message retrieval for conversation history shall complete in <500ms.

**Rationale:** Minimize LLM API delays.
**Measurement:** Query execution time on indexed session_id.

#### NFR-004: Telegram API Polling Efficiency
**Requirement:** Polling interval shall be 1 second (configurable).

**Rationale:** Balance responsiveness vs. API rate limits.
**Measurement:** Time between consecutive `getUpdates` calls.

### 5.2 Safety and Availability

#### NFR-005: System Availability
**Requirement:** Telegram integration shall maintain 99.0% availability (uptime) measured monthly.

**Rationale:** Production system reliability.
**Exclusions:** Telegram API outages, third-party provider outages.
**Measurement:** Uptime percentage = (total_time - downtime) / total_time.

#### NFR-006: Graceful Degradation
**Requirement:** If LLM provider is unavailable, users shall receive error message within 5 seconds (not hang indefinitely).

**Rationale:** Prevent poor user experience and resource exhaustion.
**Implementation:** Hard timeout on LLM calls, immediate error response.

#### NFR-007: Resource Exhaustion Protection
**Requirement:** System shall not accumulate unbounded message queues or connections.

**Rationale:** Prevent memory leaks and cascading failures.
**Implementation:**
- Max queue size with oldest-message-drop on overflow
- Connection pooling with maximum pool size
- Automatic session cleanup after 30 days of inactivity

### 5.3 Security Requirements

#### NFR-008: User Access Control
**Requirement:** Only whitelisted Telegram user IDs shall receive agent responses; non-whitelisted messages shall be rejected.

**Acceptance Criteria:**
- [ ] Whitelist check occurs before any LLM processing
- [ ] No agent response generated for non-whitelisted users
- [ ] Rejection logging includes user ID, agent ID, timestamp

#### NFR-009: Token Security
**Requirement:** Telegram bot tokens shall be encrypted at rest and never logged in plaintext.

**Acceptance Criteria:**
- [ ] Tokens encrypted using ChaCha20-Poly1305 AEAD encryption (matching existing LLM provider implementation)
- [ ] Encryption key (ENCRYPTION_KEY environment variable, 256-bit) stored separately from database
- [ ] Log files contain no unencrypted tokens
- [ ] Token masked in UI (show only last 4 characters)
- [ ] Each encryption uses a cryptographically random 96-bit nonce
- [ ] Nonce prepended to ciphertext and stored as hex in database

#### NFR-010: API Key Protection
**Requirement:** LLM provider API keys and Telegram bot tokens shall use identical encryption implementation.

**Acceptance Criteria:**
- [ ] Both token types encrypted using ChaCha20-Poly1305 via shared `Cipher` implementation (`src/crypto.rs`)
- [ ] Keys not exposed in error messages (generic error responses to clients)
- [ ] Key rotation procedure: Update ENCRYPTION_KEY environment variable, re-encrypt data by updating each provider
- [ ] Reuse existing encryption/decryption logic from `llm_providers` module pattern

#### NFR-011: Audit Logging
**Requirement:** All Telegram interactions shall be logged to console for debugging and monitoring.

**Acceptance Criteria:**
- [ ] Logs record: timestamp, user ID, agent ID, status, outcome
- [ ] Structured logging format (using tracing crate, consistent with existing application)
- [ ] Logs output to console (log retention managed by deployment environment)
- [ ] Error cases logged with full context for troubleshooting

#### NFR-012: Injection Attack Prevention
**Requirement:** System shall prevent injection attacks (SQL injection, prompt injection, command injection).

**Acceptance Criteria:**
- [ ] Parameterized queries used for all database access
- [ ] User messages treated as untrusted data, never concatenated into queries
- [ ] System prompts immutable (not constructed from user input)
- [ ] JSON parsing with strict schemas

#### NFR-013: Rate Limiting
**Requirement:** No explicit rate limiting implemented. Rely on Telegram API rate limits and LLM provider throttling.

**Rationale:** Simplifies implementation; rate limiting enforced at external service level.
**Note:** Future enhancement if abuse detected in production.

### 5.4 Reliability and Recoverability

#### NFR-014: Error Resilience
**Requirement:** Transient failures shall not cause message loss or session corruption.

**Acceptance Criteria:**
- [ ] Database transactions ACID-compliant
- [ ] Failed LLM calls do not corrupt session state
- [ ] Duplicate message detection prevents re-processing
- [ ] Partial failures (e.g., message saved but Telegram send fails) are handled

#### NFR-015: Message Durability
**Requirement:** All messages shall be persisted to database before acknowledgment to Telegram.

**Acceptance Criteria:**
- [ ] Write-through: DB commit before Telegram sendMessage call
- [ ] Failed DB transactions result in message rejection, not silent drop
- [ ] Administrator can retrieve all message history

#### NFR-016: Backup and Recovery
**Requirement:** Telegram configuration and message history shall be included in database backup/restore procedures.

**Acceptance Criteria:**
- [ ] Backup includes: telegram_configs, telegram_whitelists, telegram_logs tables
- [ ] Recovery procedures documented
- [ ] Point-in-time recovery supported (30-day retention minimum)

### 5.5 Maintainability and Testability

#### NFR-017: Code Quality
**Requirement:** Telegram integration code shall adhere to project standards.

**Acceptance Criteria:**
- [ ] Functions encapsulate complexity (max 9 if-statements per function)
- [ ] Strict type checking (Rust compiler verification)
- [ ] Error handling at all system boundaries
- [ ] Functional programming style preferred (map/reduce over loops)

#### NFR-018: Logging and Observability
**Requirement:** All operations shall be logged for operational visibility.

**Acceptance Criteria:**
- [ ] Info level: Message received, sent, whitelist check
- [ ] Warn level: Retries, rate limits, timeouts
- [ ] Error level: API failures, database errors, parsing errors
- [ ] Debug level: Full message content, intermediate states
- [ ] Logs include structured fields: timestamp, level, agent_id, user_id, request_id

#### NFR-019: Configuration
**Requirement:** System behavior shall be configurable without code changes.

**Acceptance Criteria:**
- [ ] Polling interval configurable
- [ ] Response timeout configurable
- [ ] Rate limit thresholds configurable
- [ ] Log retention period configurable
- [ ] Configuration loaded from environment variables or config file

### 5.6 Compatibility and Portability

#### NFR-020: Telegram API Compatibility
**Requirement:** System shall remain compatible with current Telegram Bot API (version 7.x).

**Rationale:** Telegram API changes may break integration.
**Mitigation:** Monitor Telegram changelog, version pinning in dependencies.

#### NFR-021: Platform Independence
**Requirement:** Telegram integration shall run on Linux, macOS, and Windows (via WSL2).

**Rationale:** Deployment flexibility.
**Implementation:** Use Rust cross-platform libraries, test on all platforms.

---

## 6. Other Requirements

### 6.1 Database Schema Requirements

#### 6.1.1 New Tables
See Section 4.3.2 for complete schema definitions:
- `telegram_configs`: Bot token and enablement status
- `telegram_whitelists`: User ID whitelists per agent
- Note: No `telegram_logs` table (logging to console only)

#### 6.1.2 Indexes
```sql
CREATE INDEX idx_telegram_whitelists_agent
  ON telegram_whitelists(agent_id);

CREATE INDEX idx_telegram_configs_enabled
  ON telegram_configs(is_enabled)
  WHERE is_enabled = true;
```

#### 6.1.3 Encryption
- Telegram bot tokens stored encrypted using ChaCha20-Poly1305 AEAD encryption (application layer)
- Matches pattern used by `llm_providers.api_key` encryption (see `src/crypto.rs`)
- Encryption handled via `Cipher::encrypt()` on write, `Cipher::decrypt()` on read
- Each encryption uses a cryptographically random 96-bit nonce prepended to ciphertext
- Entire encrypted value (nonce + ciphertext) hex-encoded for database storage
- Encryption key (`ENCRYPTION_KEY`) must be 256 bits (32 bytes, 64 hex characters)
- Key validation occurs at application startup in `crypto::Cipher::new()`

### 6.2 Deployment Requirements

#### 6.2.1 Environment Variables
```
TELEGRAM_POLLING_INTERVAL_SECONDS=1
TELEGRAM_MESSAGE_TIMEOUT_SECONDS=120
ENCRYPTION_KEY=<64-hex-characters>
```

**Notes:**
- `ENCRYPTION_KEY` is shared with LLM provider API key encryption. Generate via:
  ```bash
  openssl rand -hex 32  # Generates 64 hex characters (256 bits)
  ```
- Rate limiting not enforced by application (managed by Telegram API and LLM providers)
- Logging to console only (no database retention)
- No retry logic (fail-fast approach)

#### 6.2.2 Database Migration
- Migration scripts provided to create new tables
- Backward compatibility: Existing agents unaffected until explicitly configured
- Migration can be rolled back safely (drops new tables only)

#### 6.2.3 Monitoring and Alerting
- Out of scope for initial implementation
- Logging to console provides visibility for operational monitoring
- Future enhancement: Add metrics collection and alerting if needed

### 6.3 Compliance and Regulatory

#### 6.3.1 Data Privacy
- **GDPR Compliance:** User data (Telegram user ID) stored per user consent
- **Data Retention:** Session and message data persisted indefinitely (administrator-controlled deletion)
- **User Rights:** Administrators can delete user-associated messages via database

#### 6.3.2 Terms of Service
- **Telegram:** No bot spam, respect rate limits, compliant with Telegram Bot API ToS
- **LLM Providers:** Comply with OpenAI/provider ToS regarding message processing

---

## 7. Appendices

### 7.1 Data Dictionary

#### telegram_configs Table
| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | UUID | NO | Primary key, auto-generated |
| agent_id | UUID | NO | Foreign key to agents table, UNIQUE |
| bot_token | VARCHAR(255) | NO | Encrypted Telegram bot token |
| is_enabled | BOOLEAN | NO | Enable/disable Telegram for this agent |
| created_at | TIMESTAMP | NO | Record creation timestamp |
| updated_at | TIMESTAMP | NO | Record last update timestamp |

#### telegram_whitelists Table
| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | UUID | NO | Primary key, auto-generated |
| agent_id | UUID | NO | Foreign key to agents table |
| telegram_user_id | BIGINT | NO | Telegram user ID (numeric) |
| created_at | TIMESTAMP | NO | Record creation timestamp |
| Note | | | Composite unique constraint on (agent_id, telegram_user_id) |

#### Logging
Interactions logged to console via tracing crate (not persisted to database). No `telegram_logs` table.

### 7.2 Traceability Matrix

| Requirement | Section | Validation Method | Test Case |
|-------------|---------|-------------------|-----------|
| FR-001 | 3.1 | Functional test | Add valid token, verify stored |
| FR-002 | 3.1 | Functional test | Remove token, verify agent inactive |
| FR-003 | 3.1 | Functional test | View config page, token masked |
| FR-004 | 3.2 | Functional test | Add/remove user IDs, verify list |
| FR-005 | 3.2 | Unit + integration | Non-whitelisted user rejected |
| FR-006 | 3.2 | Unit test | Console log entry created for each attempt |
| FR-007 | 3.3 | Unit test | Message received from Telegram API (mock) |
| FR-008 | 3.3 | Unit test | Empty/long messages rejected |
| FR-009 | 3.3 | Unit test | Message routed to correct agent |
| FR-010 | 3.4 | Unit test | LLM API called with correct context |
| FR-011 | 3.4 | Unit test | Response sent via Telegram sendMessage |
| FR-012 | 3.4 | Unit test | Conversation history maintained in DB |
| FR-013 | 3.5 | Unit test | HTTP errors caught, user notified (no retries) |
| FR-014 | 3.5 | Unit test | LLM timeout (2 min) handled, user notified |
| FR-015 | 3.5 | Unit test | DB errors caught, state consistent |
| NFR-001 | 5.1 | Unit test | LLM timeout = 2 minutes |
| NFR-004 | 5.1 | Unit test | Polling interval = 1 second |
| NFR-009 | 5.3 | Unit test | Token encryption (ChaCha20-Poly1305) verified |
| NFR-011 | 5.3 | Unit test | Console logging working |

### 7.3 Architecture Diagrams

#### 7.3.1 Telegram Integration Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│         Agent Builder Application (Rust/Axum 0.7)            │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │   Telegram Integration Module (telegram/)            │  │
│  │   Following existing pattern: mod/db/service         │  │
│  │                                                       │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │  │
│  │  │   Poller     │  │   Access     │  │  Response  │ │  │
│  │  │   (getUpdates│─►│   Control    │─►│   Sender   │ │  │
│  │  │   polling)   │  │  (Whitelist) │  │ (sendMsg)  │ │  │
│  │  └──────────────┘  └──────────────┘  └────────────┘ │  │
│  │         │                │                   │        │  │
│  │         ▼                ▼                   ▼        │  │
│  │  ┌──────────────────────────────────────────────┐   │  │
│  │  │    Existing Services (Reuse)                 │   │  │
│  │  │  - agents/service.rs                         │   │  │
│  │  │  - sessions/service.rs                       │   │  │
│  │  │  - messages/service.rs                       │   │  │
│  │  │  - llm/service.rs (for LLM calls)           │   │  │
│  │  └──────────────────────────────────────────────┘   │  │
│  │         │                                            │  │
│  │         ▼                                            │  │
│  │  ┌──────────────────────────────────────────────┐   │  │
│  │  │    Encryption Layer (src/crypto.rs)          │   │  │
│  │  │  - Cipher (ChaCha20-Poly1305)               │   │  │
│  │  │  - Shared with llm_providers                 │   │  │
│  │  └──────────────────────────────────────────────┘   │  │
│  │         │                                            │  │
│  │         ▼                                            │  │
│  │  ┌──────────────────────────────────────────────┐   │  │
│  │  │   PostgreSQL Database                        │   │  │
│  │  │   - telegram_configs (encrypted tokens)      │   │  │
│  │  │   - telegram_whitelists                      │   │  │
│  │  │   - telegram_logs                            │   │  │
│  │  │   - sessions (reuse)                         │   │  │
│  │  │   - messages (reuse, ON DELETE CASCADE)      │   │  │
│  │  └──────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
│              ▲                        ▲                     │
│              │                        │                     │
└──────────────┼────────────────────────┼──────────────────────┘
               │                        │
        ┌──────▼──────┐        ┌────────▼──────┐
        │  Telegram   │        │  LLM Provider │
        │   Bot API   │        │   (OpenAI)    │
        │ (getUpdates,│        │   /v1/chat/   │
        │ sendMessage)│        │ completions   │
        └─────────────┘        └────────────────┘
```

**Codebase Structure:**
```
src/
├── telegram/              # NEW: Telegram integration module
│   ├── mod.rs            # Module exports
│   ├── db.rs             # telegram_configs, telegram_whitelists, telegram_logs queries
│   └── service.rs        # Business logic: token management, polling, message routing
├── crypto.rs             # EXISTING: Cipher struct (ChaCha20-Poly1305 encryption/decryption)
├── agents/               # EXISTING: Agent management (reuse for Telegram agents)
├── sessions/             # EXISTING: Session management (one session per Telegram user-agent pair)
├── messages/             # EXISTING: Message storage (cascade delete when session deleted)
└── llm/                  # EXISTING: LLM provider calls (call LLM for agent responses)
```

#### 7.3.2 Message Processing State Diagram

```
           ┌─────────────────────────────────────┐
           │   Telegram Message Received         │
           └──────────────────┬──────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │  Parse Message   │
                    │  & Validate      │
                    └────────┬─────────┘
                             │
                        ┌────┴─────────┐
                        │              │
              Invalid ◄─┘              └──► Valid
                (drop,              (continue)
                 log)                   │
                                        ▼
                            ┌───────────────────┐
                            │ Check Whitelist   │
                            │ (agent, user_id)  │
                            └────────┬──────────┘
                                     │
                                ┌────┴──────────┐
                                │               │
                    Denied ◄────┘               └────► Approved
                  (user msg                      (continue)
                   to log,                          │
                   reject)                         ▼
                                        ┌──────────────────────┐
                                        │ Load Session & Call  │
                                        │ LLM Provider         │
                                        └────────┬─────────────┘
                                                 │
                                            ┌────┴──────────┐
                                            │               │
                                     Success ◄┘              └──► Error
                                        │                  (log, notify user)
                                        ▼
                            ┌───────────────────────┐
                            │ Save Response & Send  │
                            │ via Telegram API      │
                            └────────┬──────────────┘
                                     │
                                ┌────┴──────────┐
                                │               │
                         Success ◄┘              └──► Error
                            │                  (retry/log/notify)
                            ▼
                    ┌──────────────────┐
                    │   Complete       │
                    │   (log success)  │
                    └──────────────────┘
```

### 7.4 Outstanding Questions and TBD Items

#### Stakeholder Decisions

1. **Polling vs. Webhook:** Use Telegram polling (getUpdates)
   - **Decision:** Polling-based approach
   - **Rationale:** Simpler implementation, fits Axum async architecture naturally
   - **Implementation:** Continuous polling loop with 1-second interval (configurable)

2. **Multi-Agent Bots:** One Telegram bot token per agent
   - **Decision:** One token per agent (not shared across agents)
   - **Rationale:** Simpler routing logic, clearer access control, matches existing agents/user model
   - **Implementation:** Agent has at most one active Telegram bot token

3. **Response Format:** Support markdown formatting
   - **Decision:** Responses support Telegram markdown format
   - **Rationale:** Enhanced user experience while maintaining simplicity
   - **Implementation:** Pass `parse_mode: "Markdown"` to Telegram sendMessage API

4. **Conversation Reset:** No automatic conversation history reset
   - **Decision:** Conversations persist indefinitely (no automatic cleanup)
   - **Rationale:** Users expect continuity; administrator can manually delete if needed
   - **Investigation TBD:** Future work to evaluate log retention and cleanup policies
   - **Implementation:** Sessions and messages remain until explicitly deleted

5. **Message Editing:** Users cannot edit messages after sending
   - **Decision:** Edited messages are treated as new messages
   - **Rationale:** Simplifies implementation, avoids redo/retry complexity
   - **Implementation:** No special handling for Telegram edit_message updates (ignored)

#### Resolved Items

- [x] Token encryption algorithm: **ChaCha20-Poly1305** (matches existing llm_providers implementation)
- [x] Polling vs. webhook: **Polling** with 1-second interval
- [x] Multi-agent bots: **One token per agent**
- [x] Response format: **Markdown support**
- [x] Conversation reset: **No automatic reset**
- [x] Message editing: **Not supported**
- [x] Rate limiting: **Not implemented** (rely on external services)
- [x] Response timeout: **2 minutes (120 seconds)**
- [x] Log retention: **Console logging only** (no database retention)
- [x] Retry strategy: **Fail-fast, no retries** (internal code handles if needed)
- [x] Error messages: **Generic, user-friendly** (no internal details)
- [x] Status dashboard: **Out of scope**
- [x] Export feature: **Out of scope**
- [x] Deployment runbook: **Out of scope**
- [x] Monitoring/alerting: **Out of scope**

### 7.5 Encryption Implementation Reference

**Implementation Pattern (from ENCRYPTION_SETUP.md):**

The Telegram bot token encryption follows the existing `llm_providers` encryption pattern:

**Architecture:**
```
Encryption Flow (Token Storage):
POST /agents/:id/telegram/config
  └─> create_telegram_config(bot_token)
      └─> Cipher::encrypt(bot_token) -> "hex_encoded_ciphertext_with_nonce"
          └─> INSERT INTO telegram_configs (bot_token) VALUES ("hex_encoded...")

Decryption Flow (Token Retrieval):
GET /agents/:id/telegram/config
  └─> get_telegram_config(agent_id)
      └─> SELECT * FROM telegram_configs WHERE agent_id = $1
          └─> Cipher::decrypt("hex_encoded...") -> "original_bot_token"
              └─> Return TelegramConfig { bot_token: "original_token", ... }
```

**Key Technical Details:**
- **Algorithm**: ChaCha20-Poly1305 (AEAD - Authenticated Encryption with Associated Data)
- **Key Size**: 256 bits (32 bytes) - stored in ENCRYPTION_KEY env var
- **Nonce**: Random 96-bit (12 bytes) generated per encryption via `rand::thread_rng()`
- **Storage Format**: Nonce (12 bytes) + Ciphertext concatenated and hex-encoded
- **Implementation File**: `src/crypto.rs` (Cipher struct with encrypt/decrypt methods)
- **Integration**: Called from `telegram/db.rs` on create/update, automatic on read

**Security Properties:**
✅ **Protected**
- Tokens are encrypted at rest in PostgreSQL
- Each encryption uses a cryptographically random nonce (prevents pattern leakage)
- Database backups contain only encrypted data

⚠️ **Not Protected**
- Tokens exist in plaintext in application memory during use
- Tokens visible in HTTP headers when calling Telegram Bot API
- Tokens visible in application logs if logged (must avoid)
- Database access + ENCRYPTION_KEY allows decryption

**Best Practices (from ENCRYPTION_SETUP.md):**
- Rotate ENCRYPTION_KEY periodically (requires re-encrypting all tokens)
- Use strong, unique ENCRYPTION_KEY per environment
- Never commit ENCRYPTION_KEY to version control
- Use environment-specific .env files
- Audit database access logs
- Monitor for unusual token access patterns

### 7.6 References and Related Documents

- [Telegram Bot API Documentation](https://core.telegram.org/bots/api)
- [Agent Builder Encryption Setup](ENCRYPTION_SETUP.md) - Implementation details
- [PostgreSQL Security Best Practices](https://www.postgresql.org/docs/current/sql-syntax.html)
- [OWASP Top 10 2023](https://owasp.org/www-project-top-ten/)
- [Rust Error Handling Best Practices](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [ChaCha20-Poly1305 AEAD Cipher](https://datatracker.ietf.org/doc/html/rfc7539)

---

## Document Change Log

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.1 | 2026-02-15 | System | Updated encryption details to match actual ChaCha20-Poly1305 implementation; aligned with codebase structure (Axum 0.7, modular architecture pattern) |
| 1.0 | 2026-02-15 | System | Initial SRS creation based on stakeholder requirements |

---

**Document Owner:** Development Team Lead
**Approvers:** (To be filled by stakeholders)
**Last Updated:** 2026-02-15
**Next Review Date:** 2026-03-15 (or upon significant requirement changes)

---

## Integration Implementation Checklist

**Phase 1: Schema & Database**
- [ ] Create `telegram_configs` table with encryption support
- [ ] Create `telegram_whitelists` table
- [ ] Add indexes for efficient queries
- [ ] Test encryption/decryption roundtrip with existing Cipher
- [ ] Note: No `telegram_logs` table needed (console logging only)

**Phase 2: Core Module (telegram/)**
- [ ] Create `src/telegram/mod.rs`
- [ ] Create `src/telegram/db.rs` (CRUD operations with encryption)
- [ ] Create `src/telegram/service.rs` (business logic)
- [ ] Integrate with existing Cipher struct
- [ ] Add validation and error handling

**Phase 3: Polling & Message Processing**
- [ ] Implement Telegram polling loop (getUpdates)
- [ ] Polling interval: 1 second (configurable)
- [ ] Implement message parsing and validation (Markdown support)
- [ ] Implement whitelist checking
- [ ] Route messages to correct agent-user session
- [ ] Log all operations to console via tracing crate

**Phase 4: Response Generation & Sending**
- [ ] Integrate with existing LLM services
- [ ] LLM timeout: 2 minutes (120 seconds hard limit)
- [ ] Implement response splitting (>4096 chars)
- [ ] Implement Telegram sendMessage API calls with Markdown parse_mode
- [ ] Fail-fast on errors (no retry logic)
- [ ] Log errors to console

**Phase 5: Admin UI (Minimal)**
- [ ] Add Telegram config form to agent management
- [ ] Add whitelist management interface
- [ ] Note: No status dashboard or export functionality

**Phase 6: Testing**
- [ ] Unit tests for encryption/decryption
- [ ] Unit tests for message parsing and validation
- [ ] Unit tests for whitelist checking
- [ ] Unit tests for response formatting
- [ ] Integration tests with database
- [ ] Note: No performance testing or deployment runbook required
