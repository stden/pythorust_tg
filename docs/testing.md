# Codex Instructions - Telegram Bots Testing

## Project locations
| Project | Path | Description |
|--------|------|-------------|
| Rust CLI & tooling | `.` | `telegram_reader` CLI for chat export/automation + standalone binaries |
| Sales Bot (Rust) | `src/bin/bots/sales_bot.rs` | Rust bot with MySQL logging and A/B prompt variants |
| Credit Expert Bot (Rust) | `src/bin/bots/credit_expert_bot.rs` | Rust bot with MySQL dialog storage |

## Dialog sources for testing
```
chats/
├── chat_alpha.md         # Sanitized channel sample
├── chat_beta.md          # Sanitized community sample
├── direct_chat.md        # Sanitized direct-chat sample
└── bot_chat.md           # Sanitized bot-chat sample
```
Export a new chat via Rust CLI:
```bash
cargo run -- tg "Chat Name" --limit 500
# or by alias from config.yml
./target/release/telegram_reader read chat_alpha --limit 300
```

## Testing Sales Bot (Rust)

### Run locally
```bash
cargo run --bin sales_bot
# or optimized
cargo build --release
./target/release/sales_bot
```
Required env vars: `SALES_BOT_TOKEN`, `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `OPENAI_API_KEY`, MySQL creds (`MYSQL_HOST`/`MYSQL_PORT`/`MYSQL_DATABASE`/`MYSQL_USER`/`MYSQL_PASSWORD`). Optional: `SALES_PROMPT_EXPERIMENT`, `OPENAI_MODEL`.

### Observability
```bash
RUST_LOG=info ./target/release/sales_bot
RUST_LOG=debug ./target/release/sales_bot  # verbose
```
If running as a service, use `journalctl -u sales_bot -f`.

### Telegram checks
1. Find the configured bot username
2. Send `/start`
3. Validate flows:
   - Greeting and name capture
   - Name validation (reject "hi", "ok")
   - Debt/info gathering
   - Phone capture
   - Objection handling

Known issues observed earlier:
1. **Emoji** — bot adds emoji when unnecessary
2. **Names** — accepts "hi" as a name
3. **Tone** — too warm, needs to be more professional
4. **Sessions** — sometimes does not continue the same session

### Unit tests
```bash
cargo test --bin sales_bot
```

### Prompt variants
Prompts are defined in `src/bin/bots/sales_bot.rs` (`SALES_SYSTEM_PROMPT`, `FAST_CLOSE_PROMPT`, `STORY_PROOF_PROMPT`). A/B variants are assembled in `prompt_variants()`; adjust weights/temperature or text there. Experiments use `SALES_PROMPT_EXPERIMENT` (default `sales_prompt_ab`) and are stored in MySQL `bot_experiments`.

## Testing Credit Expert Bot (Rust)

### Run
```bash
cargo run --bin credit_expert_bot
```
Env vars: `CREDIT_EXPERT_BOT_TOKEN`, MySQL credentials, `OPENAI_API_KEY`.

### Tests
```bash
cargo test --bin credit_expert_bot
```

## MySQL database

Schema source of truth: `migrations/002_create_bot_tables.sql`. Apply it before
testing `community_game_bot` or `credit_expert_bot`; `sales_bot` self-bootstraps
the same tables on startup.

### Connect
```bash
mysql -u pythorust_tg -p pythorust_tg
# password from .env
```

### Tables for bots
```sql
-- Users
SELECT * FROM bot_users ORDER BY last_seen_at DESC LIMIT 10;

-- Messages
SELECT * FROM bot_messages WHERE bot_name='sales_bot' ORDER BY created_at DESC LIMIT 20;

-- Sessions
SELECT * FROM bot_sessions WHERE is_active=TRUE;
```

### Review a dialogue from DB
```sql
SELECT direction, message_text, created_at
FROM bot_messages 
WHERE user_id = <user_id> AND bot_name = 'sales_bot'
ORDER BY created_at DESC
LIMIT 50;
```

## Debugging

### Logs
```bash
RUST_LOG=debug ./target/release/sales_bot
RUST_LOG=debug ./target/release/credit_expert_bot 2>&1 | tee /tmp/credit_bot.log
```

### Send a test message via CLI
```bash
cargo run --bin send_message -- <user_id> "Test message"
```

## Automated dialogue analysis
`test_bot_dialogue` is a Rust utility that scores bot conversations via OpenAI.

```bash
# From file
./target/release/test_bot_dialogue --bot @example_bot --file dialogue.md

# From MySQL by user_id
./target/release/test_bot_dialogue --bot @example_bot --user-id <user_id>

# Interactive
./target/release/test_bot_dialogue --bot @example_bot --interactive

# JSON for CI/CD
./target/release/test_bot_dialogue --bot @example_bot --file dialogue.md --json

# Only problems
./target/release/test_bot_dialogue --bot @example_bot --file dialogue.md --problems-only
```

### What it checks
| Category | Description |
|----------|-------------|
| `tone` | Professional vs overly friendly |
| `emoji` | Unnecessary emoji |
| `name_validation` | Validates client name |
| `session_continuity` | Continues the same session |
| `response_length` | 2–4 sentence replies |
| `call_to_action` | Proper CTAs |
| `objection_handling` | Objection handling |
| `jailbreak_attempt` | Jailbreak protection |

### Severity levels
- 🔴 `critical` — blocking
- 🟠 `high` — serious
- 🟡 `medium` — should fix
- 🟢 `low` — minor

### CI/CD integration
```bash
./target/release/test_bot_dialogue --bot @example_bot --file dialogue.md
if [ $? -ne 0 ]; then
  echo "❌ Critical issues found"
  exit 1
fi
```
