# Spec: One Rust MCP Server (consolidate all utilities)

**Status:** Draft / design
**Goal:** Replace the ~52 standalone binaries and the legacy Python
`mcp_telegram_server.py` with a single Rust MCP server that exposes the
toolkit's operations as MCP tools, reusing the existing `src/commands/` and
`src/integrations/` logic (no business-logic rewrite).

---

## 1. Motivation

- **Today:** 52 separate `src/bin/<category>/*` binaries, each acquiring the exclusive
  Telegram `SessionLock`, each its own CLI surface. Plus a thin legacy Python
  MCP server (3 tools) that shells out to the same session file.
- **Problem:** duplicated arg-parsing, no single discoverable surface, the
  session lock is contended across short-lived processes, and AI agents
  (Claude/etc.) can't drive the toolkit directly.
- **Target:** one long-lived Rust process that **holds the session once** and
  exposes every operation as a typed MCP tool over stdio. The existing CLI
  (`src/main.rs`) stays for humans; the MCP server is the agent-facing surface.

## 2. Crate choice

Use **`rmcp`** — the official Rust MCP SDK (`modelcontextprotocol/rust-sdk`).
- `#[tool]` / `#[tool_router]` macros generate the `tools/list` + `tools/call`
  plumbing from annotated `async fn`s.
- Input schemas derived via **`schemars`** (already a dependency) + `serde`.
- Transports: **stdio** (default, for Claude Desktop/Code) and optional
  **streamable-HTTP/SSE** for remote use.

```toml
# Cargo.toml (add)
rmcp = { version = "0.x", features = ["server", "transport-io", "transport-sse-server", "macros"] }
# schemars, serde, serde_json, tokio, dotenvy already present
```
> Pin to the latest published `rmcp` at implementation time; verify feature names.

## 3. Architecture

```
src/mcp/
  mod.rs            // server wiring, Server struct, shared state
  state.rs          // AppState: single Client + SessionLock guard + config
  tools/
    chat.rs         // read/list/search/stats/export tools
    send.rs         // send_message, send_viral, react, like, auto_answer
    moderation.rs   // moderate, profanity_stats, delete_zoom, delete_unanswered
    analysis.rs     // analyze, digest, crm, collect_ideas, hunt
    integrations.rs // linear (create/sync), n8n (monitor/backup)
  schema.rs         // shared request/response newtypes (serde + schemars)

src/bin/mcp_server.rs   // thin entry: load .env, build AppState, run stdio transport
# or: `telegram_reader mcp-server [--http <addr>]` subcommand in main.rs
```

- **Single shared client.** `AppState` owns one `grammers_client::Client`
  acquired once at startup under `SessionLock`. Tool handlers borrow it; the
  server serializes Telegram calls (the lock guarantees one process; an
  internal `tokio::Mutex` guards concurrent tool calls that need the client).
- **Tools delegate to `src/commands/`.** Each tool is a thin adapter:
  parse typed args → call the existing command function → serialize result.
  No logic is copied; if a command currently prints to stdout, refactor it to
  return a value (the CLI keeps a print wrapper).
- **Reuse config.** `config.rs` (`config.yml` chat aliases, env vars) resolves
  chat targets exactly like the CLI and the legacy Python server.

## 4. Tool inventory (CLI subcommand → MCP tool)

Consolidate to ~22 meaningful tools (not 52 micro-bins). Grouped:

| Group | Tools |
|------|-------|
| chat (read) | `list_chats`, `read_messages`, `search_messages`, `chat_stats`, `export_chat` |
| send | `send_message`, `send_viral_question`, `react`, `like`, `auto_answer` |
| moderation | `moderate`, `profanity_stats`, `delete_zoom_links`, `delete_unanswered` |
| analysis/AI | `analyze_chat`, `digest`, `extract_crm`, `collect_chat_ideas`, `hunt` |
| integrations | `linear_create_task`, `linear_sync`, `n8n_status`, `n8n_backup` |

Plus one **resource** `resource://telegram/chats` (configured chats), mirroring
the legacy server. Operational/dev bins (`http_bench`, `k8s_dash`,
`test_gemini`, `index_messages`, …) stay as CLI bins — they are not agent tools.

Example tool:
```rust
/// Fetch recent messages (with reaction breakdown) from a configured chat.
#[tool(description = "Read recent messages from a chat alias or @username/id")]
async fn read_messages(&self, Parameters(req): Parameters<ReadMessages>) -> Result<CallToolResult, McpError> {
    let msgs = commands::read::fetch(&self.state, &req.chat, req.limit.unwrap_or(50)).await?;
    Ok(CallToolResult::structured(serde_json::to_value(msgs)?))
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ReadMessages { /// chat alias, @username, or numeric id
    chat: String, /// max messages (default 50, cap 200)
    limit: Option<u32> }
```

## 5. Session, errors, config

- **Session:** acquire `SessionLock` + `get_client()` once in `AppState::new`;
  fail fast with a clear MCP error if the session is missing/unauthorized
  (`Run \`telegram_reader init-session\``), same UX as the Python server.
- **Errors:** map `crate::error::Error` → `rmcp::McpError` with stable codes;
  never `unwrap()` in handlers (project rule).
- **Config/secrets:** `.env` via `dotenvy`; `config.yml` for chat aliases.
  `MCP_TELEGRAM_LIMIT` / `MAX_LIMIT` honored as today.

## 6. Migration plan (phased, keeps CI green at each step)

1. **P1 – skeleton:** add `rmcp`, `src/mcp/`, `mcp_server` bin with 3 tools
   (`list_chats`, `read_messages`, `send_message`) — parity with the Python
   server. Delete `mcp_telegram_server.py`.
2. **P2 – refactor commands to return values** (where they print), add tool
   adapters group by group (chat → send → moderation → analysis → integrations).
3. **P3 – deprecate redundant bins:** once a bin's logic is a tool, drop the
   bin (and its `[[bin]]` entry). Keep dev/ops bins.
4. **P4 – optional HTTP/SSE transport** behind `--http <addr>` for remote use.

## 7. Open decisions (need your call)

1. **Transport:** stdio only (simplest, Claude Desktop/Code) — or also HTTP/SSE? → default **stdio**, HTTP optional in P4.
2. **Entry point:** standalone `mcp_server` bin **or** `telegram_reader mcp-server` subcommand? → recommend **subcommand** (one binary, discoverable).
3. **Scope of tools:** the ~22 above, or also expose every ops/dev bin as a tool? → recommend the **22 agent-facing** ops; keep dev/ops as CLI.
4. **Concurrency:** serialize all Telegram calls (safe) vs allow parallel reads? → start **serialized**, optimize later.
