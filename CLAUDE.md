# Telegram Automation Toolkit (Rust)

**Important:** All new backend work must be in Rust. New features and services are Rust-only; Python is kept only for legacy scripts that are being rewritten.

## Development rules
1. Rust only for new functionality
2. Migrate remaining Python to Rust over time
3. Optimize builds (`--release`, LTO) where it matters
4. Prefer zero-cost abstractions
5. Async/await with tokio

## Rust best practices

### Quality tooling
```bash
cargo fmt --all                 # format (run before commits)
cargo clippy --all-targets -- -W clippy::pedantic  # lint
cargo audit                     # dependency vulnerability scan
```

### Release optimization (Cargo.toml)
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 0
incremental = true

[profile.dev.package."*"]
opt-level = 2
```

### Idiomatic patterns
```rust
// Prefer iterators over manual loops
let sum: i32 = values.iter().filter(|x| **x > 0).sum();

// Use Result/Option instead of panic
fn process() -> Result<Data, Error> {
    let data = fetch_data()?;
    Ok(transform(data))
}

// Avoid unnecessary clones, pass references
fn process(data: &Data) -> &str { /* ... */ }

// Use const for compile-time values
const MAX_RETRIES: u32 = 3;

// Derive standard traits
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config { /* ... */ }
```

### Safety
- Avoid `unsafe` unless it is documented and essential
- Validate inputs at boundaries
- Use `secrecy`/newtypes for sensitive data where applicable
- Never commit secrets

## CLI binary
```bash
cargo build --release
./target/release/telegram_reader --help
```

## Available commands (`telegram_reader`)
| Command | Description | Example |
|---------|-------------|---------|
| `read` | Export chat to Markdown | `telegram_reader read chat_alpha --limit 3000 --delete-unengaged` |
| `tg` | Simple export | `telegram_reader tg chat_alpha --limit 200` |
| `list-chats` | List chats | `telegram_reader list-chats --limit 20` |
| `active-chats` | Most active chats | `telegram_reader active-chats --limit 20` |
| `dialogs` | Dialog metadata | `telegram_reader dialogs --limit 50 --format json` |
| `export` | Export by username | `telegram_reader export username --limit 300 --output chat.md` |
| `delete-zoom` | Remove Zoom links | `telegram_reader delete-zoom username --limit 3000` |
| `analyze` | AI chat analysis | `telegram_reader analyze @channel --limit 800 --days 30` |
| `auto-answer` | AI auto-responder | `telegram_reader auto-answer --model gpt-4o-mini` |
| `init-session` | Initialize session | `telegram_reader init-session` |
| `linear` | Create Linear issue | `telegram_reader linear --title "Bug" --description "Steps"` |
| `digest` | AI digest | `telegram_reader digest chat_alpha -H 24 --limit 500` |
| `moderate` | Profanity moderation | `telegram_reader moderate chat_alpha --delete --warn` |
| `profanity-stats` | Profanity stats | `telegram_reader profanity-stats chat_alpha --limit 1000` |
| `crm` | CRM parsing | `telegram_reader crm chat_alpha --limit 100 --export-csv contacts.csv` |
| `hunt` | Keyword-based user hunt | `telegram_reader hunt --chats chat1,chat2 --keywords "jobs" --required "python"` |
| `like` | Like messages from a user | `telegram_reader like --chat chat_alpha --user target --emoji "❤️"` |
| `react` | React to ids/links/recent messages | `telegram_reader react --chat chat_alpha --ids 123,124 --emoji "🔥"` |
| `send-viral` | Send configured viral questions | `telegram_reader send-viral` |
| `n8n-monitor` | Monitor N8N | `telegram_reader n8n-monitor` |
| `n8n-backup` | N8N backup | `telegram_reader n8n-backup backup` |

## Project structure
```
.
├── Cargo.toml              # Rust dependencies and profiles
├── src/
│   ├── main.rs             # CLI entrypoint
│   ├── lib.rs              # Library exports
│   ├── chat.rs             # Chat parsing helpers
│   ├── session.rs          # Grammers session + locking
│   ├── error.rs            # Error types
│   ├── metrics.rs          # Prometheus server
│   ├── commands/           # CLI commands
│   │   ├── read.rs         # Chat reading
│   │   ├── tg.rs           # Simple export
│   │   ├── like.rs         # Reactions
│   │   ├── digest.rs       # AI digest
│   │   ├── moderate.rs     # Moderation
│   │   ├── crm.rs          # CRM parsing
│   │   ├── hunt.rs         # User hunt
│   │   ├── n8n.rs          # N8N monitor/backup
│   │   ├── linear.rs       # Linear integration
│   │   ├── analyze.rs      # AI analyzer
│   │   ├── react.rs        # Reaction sender
│   │   └── ...             # Other helpers
│   ├── integrations/       # AI providers (OpenAI/Gemini/Claude/Ollama)
│   ├── analysis/           # Vector DB / graph DB helpers (Qdrant, Neo4j)
│   ├── lightrag/           # LightRAG support
│   └── prompts.rs          # Prompt loading utilities
├── src/bin/                # Standalone binaries (delete_unanswered, lightrag, http_bench, k8s_dash, bfl_sales_bot, etc.)
├── prompts/                # System prompts (Markdown)
├── config.yml              # Chat configuration
└── codev/                  # Specs, plans, protocols, reviews
```

## Rust sources
| File | Description |
|------|-------------|
| src/main.rs | Clap-based CLI entrypoint |
| src/lib.rs | Library exports |
| src/config.rs | YAML config loading |
| src/session.rs | Grammers session handling |
| src/commands/read.rs | Chat reading/export |
| src/commands/tg.rs | Simple export |
| src/commands/like.rs | Reactions |
| src/commands/digest.rs | AI digest |
| src/commands/moderate.rs | Moderation |
| src/commands/crm.rs | CRM parsing |
| src/commands/hunt.rs | User hunt |
| src/commands/n8n.rs | N8N monitoring/backup |
| src/commands/linear.rs | Linear integration |
| src/integrations/openai.rs | OpenAI API |
| src/integrations/gemini.rs | Google Gemini |
| src/prompts.rs | Prompt loading |

## Key dependencies (Cargo.toml)
```toml
grammers-client = "0.8"     # Telegram MTProto
tokio = { version = "1", features = ["full", "signal"] }
clap = { version = "4", features = ["derive", "env"] }
async-openai = "0.28"      # OpenAI API
reqwest = "0.12"            # HTTP client
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"             # Logging
prometheus = { version = "0.13", features = ["process"] }
qdrant-client = "1.16"      # Vector DB
neo4rs = "0.8"              # Graph DB
mysql_async = { version = "0.34", features = ["chrono"] } # MySQL for bots
teloxide = "0.12"           # Bot framework
kube = { version = "0.98", features = ["client", "config", "runtime", "derive"] }
```

## Environment setup
```bash
# Telegram
TELEGRAM_API_ID=12345
TELEGRAM_API_HASH=abc123
TELEGRAM_PHONE=+79001234567
MY_ID=7098803

# AI
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini

# N8N
N8N_URL=https://n8n.example.com
N8N_API_KEY=...
N8N_RESTART_COMMAND="systemctl restart n8n"
BACKUP_DIR=/srv/backups/n8n
```

Initialize the session (one time):
```bash
./target/release/telegram_reader init-session
# Enter YES and the Telegram code
```

## Python (legacy) — do not use for new code
Scripts to migrate to Rust:
| Python | Migration status |
|--------|------------------|
| `n8n_monitor.py` | ✅ Replaced by `n8n-monitor` |
| `n8n_backup.py` | ✅ Replaced by `n8n-backup` |
| `chat_analysis` | ❌ Pending migration |
| `mcp_telegram_server.py` | ❌ Pending migration |
| `ai_project_consultant.py` | ❌ Pending migration |
| `bfl_sales_bot.py` | ❌ Pending migration (Rust bot exists) |
| `credit_expert_bot.py` | ❌ Pending migration |

## Build and run
```bash
# Dev build
cargo build

# Optimized build
cargo build --release

# Maximized optimization
cargo build --profile release-optimized

# Run with logs
RUST_LOG=info ./target/release/telegram_reader list-chats

# Tests
cargo test

# Benchmarks
cargo bench
```

## CI/CD checks
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

**Priorities: performance and safety. Rust lets us match C++ speed with memory safety guarantees.**
