# TELEGRAM_ACCESS_SRS Corrections Made

## Summary
Updated TELEGRAM_ACCESS_SRS v1.0 → v1.1 to align with:
1. Actual codebase architecture (Axum 0.7, modular pattern)
2. Existing ChaCha20-Poly1305 encryption implementation
3. Current PostgreSQL schema patterns

---

## Detailed Corrections

### 1. Encryption Algorithm Specification

**Original (Incorrect):**
```
NFR-009: "Tokens encrypted using AES-256-GCM or equivalent"
NFR-010: "Keys encrypted at rest" (vague)
Section 6.1.3: "database-level encryption (e.g., pgcrypto)"
```

**Corrected:**
```
NFR-009: "Tokens encrypted using ChaCha20-Poly1305 AEAD encryption
          (matching existing LLM provider implementation)"
NFR-010: "Both token types encrypted using ChaCha20-Poly1305 via
          shared Cipher implementation (src/crypto.rs)"
Section 6.1.3: "ChaCha20-Poly1305 AEAD encryption (application layer)
                Matches pattern used by llm_providers.api_key encryption"
```

**Why:** The codebase already uses ChaCha20-Poly1305 from the chacha20poly1305 crate. No AES encryption is implemented or planned. The Cipher struct in src/crypto.rs is the single source of truth.

---

### 2. Encryption Key Management

**Original (Incorrect):**
```
Section 6.2.1: "DATABASE_ENCRYPTION_KEY=<32-byte-hex>"
```

**Corrected:**
```
Section 6.2.1: "ENCRYPTION_KEY=<64-hex-characters>"

Note: ENCRYPTION_KEY is shared with LLM provider API key encryption.
Generate via: openssl rand -hex 32
```

**Why:**
- The environment variable is named `ENCRYPTION_KEY` (not `DATABASE_ENCRYPTION_KEY`)
- It's shared between llm_providers and telegram modules
- Display format: 64 hex characters (256 bits)

---

### 3. Database Schema Definition

**Original (Incorrect):**
```sql
bot_token VARCHAR(255) NOT NULL ENCRYPTED,
-- Missing column type for encryption
-- INDEX syntax incorrect for PostgreSQL
INDEX (agent_id, created_at),
```

**Corrected:**
```sql
bot_token TEXT NOT NULL,
-- Encryption handled by application layer (Cipher struct)
-- Explicit index creation syntax:

CREATE INDEX IF NOT EXISTS idx_telegram_logs_agent_timestamp
  ON telegram_logs(agent_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_telegram_logs_user_timestamp
  ON telegram_logs(telegram_user_id, created_at DESC);
```

**Why:**
- PostgreSQL has no `ENCRYPTED` keyword in table definitions
- Encryption is application-layer (ChaCha20-Poly1305), not database-layer
- Matches pattern used in llm_providers table
- Uses proper PostgreSQL syntax for indexes

---

### 4. Timestamp Column Types

**Original (Inconsistent):**
```sql
created_at TIMESTAMP NOT NULL DEFAULT NOW(),
updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
```

**Corrected:**
```sql
created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
```

**Why:** Matches existing Agent Builder schema pattern (see schema.rs). TIMESTAMPTZ preserves timezone information.

---

### 5. Missing Encryption Implementation Details

**Added (New Sections):**

**Section 1.3** - Added to Definitions:
- ChaCha20-Poly1305: AEAD encryption algorithm
- Nonce: Random 96-bit value per encryption
- ENCRYPTION_KEY: 256-bit encryption key

**Section 4.3.2** - Added to PostgreSQL:
```
Encryption Details:
- bot_token field stores hex-encoded ciphertext (nonce + encrypted token)
- Encryption/decryption handled by Cipher struct from src/crypto.rs
- Pattern matches existing llm_providers.api_key encryption
- Requires ENCRYPTION_KEY environment variable
```

**Section 6.1.3** - Added to Encryption:
- Detailed algorithm specification (ChaCha20-Poly1305)
- Key validation (256 bits = 64 hex chars)
- Nonce generation (random, 12 bytes)
- Storage format (hex-encoded)
- Integration pattern (call from telegram/db.rs)

**Section 7.5** - NEW: Encryption Implementation Reference
- Detailed flow diagrams (encryption/decryption)
- Technical specifications
- Security properties
- Best practices from ENCRYPTION_SETUP.md

---

### 6. Codebase Architecture Alignment

**Added Documentation:**

**Section 7.3.1** - Component Diagram:
- Detailed Axum 0.7 application structure
- Module relationships (telegram → agents, sessions, messages)
- Encryption layer placement
- Reuse of existing services

**New Code Structure:**
```
Added to SRS:
src/telegram/              # NEW: Telegram integration module
  ├── mod.rs
  ├── db.rs              # CRUD with encryption
  └── service.rs         # Business logic

Reuse existing:
src/crypto.rs            # Cipher (ChaCha20-Poly1305)
src/agents/              # Agent management
src/sessions/            # Session model
src/messages/            # Message storage
src/llm/                 # LLM provider calls
```

---

### 7. Framework-Specific Details

**Original (Generic):**
```
"Rust web application framework"
```

**Corrected (Specific):**
```
"Rust web application framework (Axum 0.7 with async request handling)"

Tech Stack Details:
- Axum 0.7: Modern async web framework
- SQLx 0.7: Async SQL toolkit with compile-time query verification
- Tokio 1.35: Async runtime
- ChaCha20-Poly1305: AEAD encryption
- Tower/Tower-HTTP: Middleware and HTTP utilities
```

**Why:** Specificity helps development team understand constraints and patterns to follow.

---

### 8. Outstanding Questions Updated

**Before:**
```
1. "Polling vs. Webhook" - generic recommendation
```

**After:**
```
1. "Polling vs. Webhook"
   Current Architecture: Axum runs as single async process - polling fits naturally
   Recommendation: Start with polling for simplicity; webhook as future optimization
```

**Similar updates to all 5 questions** - now includes codebase context.

---

### 9. TBD Items Resolved

**Original:**
```
- [ ] Final token encryption algorithm (AES-256-GCM vs. alternatives)
- [ ] Exact rate limit thresholds (currently 10 messages/min/user)
```

**Corrected:**
```
- [x] Token encryption algorithm: ChaCha20-Poly1305 (matches existing llm_providers)
- [ ] Exact rate limit thresholds (currently 10 messages/min/user)
```

---

### 10. New Content Added

**Section 7.6** - References and Related Documents:
```
Added:
- [Agent Builder Encryption Setup](ENCRYPTION_SETUP.md)
- [ChaCha20-Poly1305 AEAD Cipher RFC](https://datatracker.ietf.org/doc/html/rfc7539)
```

**New Section** - Integration Implementation Checklist
```
6 Phases:
1. Schema & Database
2. Core Module (telegram/)
3. Polling & Message Processing
4. Response Generation & Sending
5. Admin UI
6. Testing & Deployment
```

**Document Change Log:**
```
Added v1.1 entry documenting all updates
```

---

## Cross-Reference Validation

✅ **crypto.rs references verified:**
- Line 61: References ChaCha20-Poly1305
- Line 589: References Cipher struct from src/crypto.rs
- Line 870: References ENCRYPTION_KEY environment variable

✅ **codebase alignment verified:**
- Axum 0.7 matches Cargo.toml
- SQLx 0.7 matches Cargo.toml
- Async pattern matches main.rs and routes.rs
- Modular pattern matches src/ directory structure

✅ **encryption implementation verified:**
- ChaCha20-Poly1305 matches chacha20poly1305 dependency in Cargo.toml
- Cipher struct exists and is functional in src/crypto.rs
- ENCRYPTION_KEY validation exists in crypto.rs:21-23
- Integration pattern mirrors llm_providers/db.rs

---

## Files Created/Modified

**Modified:**
- TELEGRAM_ACCESS_SRS.md (updated to v1.1)

**Created:**
- SRS_UPDATE_SUMMARY.md (this document's companion)
- CORRECTIONS_MADE.md (this file)

---

## Verification Steps Completed

1. ✅ Verified ChaCha20-Poly1305 in Cargo.toml
2. ✅ Verified ENCRYPTION_KEY usage in crypto.rs
3. ✅ Verified llm_providers encryption pattern
4. ✅ Verified Axum 0.7 framework usage
5. ✅ Verified PostgreSQL schema patterns in schema.rs
6. ✅ Verified module structure matches src/ directory
7. ✅ Verified async/await patterns throughout codebase

---

**Status:** ✅ All corrections complete and verified
**SRS Version:** 1.1
**Updated:** 2026-02-15
