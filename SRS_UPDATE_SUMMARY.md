# TELEGRAM_ACCESS_SRS Update Summary

**Date:** 2026-02-15
**Updated Version:** 1.1

## Overview
The TELEGRAM_ACCESS_SRS has been updated to align with the actual Agent Builder codebase structure and reflect the existing ChaCha20-Poly1305 encryption implementation used for API key storage.

## Key Changes

### 1. Encryption Implementation (Critical Updates)
**Before:** Referenced generic "AES-256-GCM or equivalent"
**After:** Specified **ChaCha20-Poly1305 AEAD encryption** matching existing `llm_providers` implementation

**Details:**
- Algorithm: ChaCha20-Poly1305 (modern, authenticated encryption)
- Key: 256-bit (32 bytes) stored in `ENCRYPTION_KEY` environment variable
- Nonce: Random 96-bit (12 bytes) per encryption operation
- Storage: Nonce + ciphertext hex-encoded in database
- Implementation: Centralized in `src/crypto.rs` Cipher struct

### 2. Environment Variables
**Before:** Referenced generic `DATABASE_ENCRYPTION_KEY`
**After:** Specified shared `ENCRYPTION_KEY` used by both:
- LLM provider API keys (existing)
- Telegram bot tokens (new)

**Generation Command:**
```bash
openssl rand -hex 32  # Produces 64 hex characters = 256 bits
```

### 3. Database Schema Updates
- Changed `bot_token VARCHAR(255) NOT NULL ENCRYPTED` to `bot_token TEXT NOT NULL` (encryption handled by application layer, not database)
- Aligned column types with existing patterns (TIMESTAMPTZ, TEXT)
- Added explicit index definitions (matching existing pattern)
- Added encryption handling documentation

### 4. Codebase Structure
**Added detailed module structure:**
```
src/telegram/              # NEW module following existing pattern
├── mod.rs
├── db.rs                  # Database operations with encryption
└── service.rs             # Business logic

src/crypto.rs             # EXISTING - shared Cipher implementation
src/agents/               # EXISTING - reuse for Telegram agents
src/sessions/             # EXISTING - one session per Telegram user-agent pair
src/messages/             # EXISTING - conversation history (ON DELETE CASCADE)
src/llm/                  # EXISTING - call LLM for agent responses
```

**Architectural Pattern:**
- Three-layer pattern (mod → db → service) matching existing modules
- Database operations via SQLx with compile-time verification
- Encryption/decryption via shared Cipher struct
- Error handling via custom AppError enum

### 5. Framework-Specific Details
**Added explicit framework versions:**
- Axum 0.7 (async web framework)
- SQLx 0.7 (async database toolkit)
- Tokio 1.35 (async runtime)
- ChaCha20-Poly1305 (cryptography library)

### 6. Security Documentation
**Added section 7.5: Encryption Implementation Reference**
- Detailed encryption/decryption flow diagrams
- Technical implementation details
- Security properties (what's protected vs. not)
- Best practices from ENCRYPTION_SETUP.md
- Key rotation procedures

### 7. Outstanding Items Resolved
| Item | Status | Resolution |
|------|--------|-----------|
| Token encryption algorithm | ✅ Resolved | ChaCha20-Poly1305 |
| Environment variable name | ✅ Resolved | ENCRYPTION_KEY (shared) |
| Database schema | ✅ Updated | Aligned with crypto layer approach |
| Codebase structure | ✅ Documented | Detailed module layout |
| Polling vs. Webhook | ⏳ TBD | Polling recommended (async-friendly) |

### 8. Implementation Checklist Added
**New section:** Integration Implementation Checklist (6 phases)
1. Schema & Database setup
2. Core module implementation (telegram/)
3. Polling & message processing
4. Response generation & sending
5. Admin UI integration
6. Testing & deployment

## Files Referenced
- `src/crypto.rs` - Cipher implementation (ChaCha20-Poly1305)
- `src/llm_providers/db.rs` - Existing encryption pattern reference
- `ENCRYPTION_SETUP.md` - Detailed encryption documentation
- `Cargo.toml` - Dependencies including chacha20poly1305 crate

## Alignment with Codebase
✅ Uses existing Cipher struct from crypto.rs
✅ Follows three-layer module pattern (mod/db/service)
✅ Leverages existing session/message models
✅ Reuses LLM service integration
✅ Shares encryption with llm_providers
✅ Maintains error handling patterns
✅ Compatible with Axum 0.7 architecture

## Notes for Development Team
1. **Encryption is not optional** - ENCRYPTION_KEY must be set at startup or application fails
2. **Shared encryption key** - Same ENCRYPTION_KEY used for both API keys and bot tokens
3. **Pattern reuse** - Follow llm_providers/db.rs and llm_providers/service.rs patterns
4. **Database layer** - Use SQLx for type-safe queries
5. **Error handling** - Use existing AppError enum for consistency
6. **Async operations** - All database calls must be async (Tokio runtime)

## Migration Path
1. Deploy with Telegram integration disabled initially
2. Verify encryption works with existing llm_providers
3. Gradually enable Telegram for selected agents
4. Monitor performance and error rates
5. Scale up as needed

---

**Document Version:** 1.1
**Last Updated:** 2026-02-15
**Next Review:** Upon implementation start or when significant changes occur
