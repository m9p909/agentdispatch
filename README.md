# AgentDispatch

A self-hosted LLM agent builder with a simple form-based UI. No visual node graphs, no complex setup — just configure your providers, models, and agents, then chat.

## Features

- **Simple UI** — providers, models, and agents are plain forms
- **Streaming agentic loop** — tool calls execute automatically and stream back to the UI in real time
- **Extensible tool system** — implement the `Tool` trait to add custom tools
- **Encrypted API keys** — provider credentials stored with ChaCha20Poly1305 encryption
- **Telegram connector** — connect agents to Telegram bots with user whitelisting
- **OpenAI-compatible** — works with any provider that speaks the OpenAI chat completions API (OpenAI, Groq, Ollama, etc.)

## Stack

- **Backend**: Rust (Axum, SQLx, Tokio)
- **Frontend**: React + TypeScript (React Router 7, React Query)
- **Database**: PostgreSQL
- **Infrastructure**: Docker Compose

## Getting Started

### Prerequisites

- Rust (stable)
- Node.js 18+
- Docker

### 1. Start the database

```bash
docker compose up -d
```

### 2. Configure environment

```bash
cp .env.example .env
```

Edit `.env`:

```env
DB_HOST=localhost
DB_PORT=5433
DB_NAME=agent_builder
DB_USER=postgres
DB_PASSWORD=postgres

HOST=0.0.0.0
PORT=8080

# Generate with: openssl rand -hex 32
ENCRYPTION_KEY=your_32_byte_hex_key_here
```

### 3. Run the backend

```bash
cargo run
```

### 4. Run the frontend

```bash
cd frontend/agent-dispatch
npm install
npm run dev
```

Open [http://localhost:5173](http://localhost:5173).

## Usage

1. **Providers** — add an LLM provider with its API base URL and key (e.g. `https://api.openai.com/v1`)
2. **Models** — register a model identifier (e.g. `gpt-4o`) against a provider
3. **Agents** — create an agent with a system prompt and a model
4. **Sessions** — start a session with an agent
5. **Chat** — send messages; tool calls and results stream inline

## Adding Custom Tools

Implement the `Tool` trait in `src/llm/tool_registry.rs` and register it in `ToolRegistry::new()`:

```rust
struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "my_tool",
                "description": "Does something useful",
                "parameters": { /* JSON Schema */ }
            }
        })
    }

    async fn execute(&self, arguments: &str) -> String {
        // your logic here
        "result".to_string()
    }
}
```

## Telegram Connector

Set up a Telegram bot token via the Telegram connectors UI. Agents can then receive and respond to messages from whitelisted Telegram users.

## License

AGPL-3.0 — see [LICENSE](LICENSE).
