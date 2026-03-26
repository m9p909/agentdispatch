# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

**Backend (Rust):**
```bash
cargo build          # Build
cargo run            # Run server
cargo test           # Run all tests
cargo test <name>    # Run single test
cargo clippy         # Lint
cargo fmt            # Format
```

**Frontend (React/TypeScript):**
```bash
cd frontend/better-agent-builder
npm run dev          # Dev server
npm run build        # Production build
npm run typecheck    # Type check
```

**Infrastructure:**
```bash
docker compose up -d  # Start PostgreSQL
```

## Architecture

Full-stack LLM agent builder: users configure providers (with encrypted API keys), models, and agents (system prompt + model), then have conversations via sessions.

**Request flow:**
```
HTTP → routes.rs → AppState.{service} → db.rs → PostgreSQL
```

**When a user sends a message:**
1. `MessageService` stores the user message
2. Fetches agent config → model → provider credentials
3. Decrypts API key via `CryptoService` (ChaCha20Poly1305)
4. Calls `LlmAdapter.call_api()` with the external LLM endpoint
5. Stores and returns the AI response

**Backend module layout** (`src/`):

Each domain (`users`, `llm_providers`, `llm_models`, `agents`, `sessions`, `messages`) has:
- `mod.rs` — types/structs
- `<name>_service.rs` — business logic + validation
- `db.rs` — SQL queries via SQLx

Cross-cutting modules: `crypto.rs` (encryption), `error.rs` (AppError → HTTP), `schema.rs` (DDL), `config.rs` (env vars), `routes.rs` (Axum route wiring).

**Frontend** (`frontend/better-agent-builder/app/`): React Router 7 with React Query for data fetching. `api.ts` is the HTTP client for all backend calls. Routes map to the five main pages: providers → models → agents → sessions → chat.

## Naming conventions

- `*Adapter` — wraps an external API (e.g. `LlmAdapter`)
- `*Service` — domain business logic (e.g. `AgentService`, `MessageService`)
- `*Db` / `db.rs` — raw SQL query functions

## Environment

Requires a `.env` file (see `.env.example` if present) with:
- `ENCRYPTION_KEY` — 32-byte hex key for API key encryption
- `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD`
- `HOST`, `PORT` — server bind address
