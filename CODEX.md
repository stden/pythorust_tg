# Codex Instructions - Telegram Bots Testing

## Project locations
| Project | Path | Description |
|--------|------|-------------|
| Rust CLI & tooling | `.` | `telegram_reader` CLI for chat export/automation + standalone binaries |
| BFL Sales Bot (Rust) | `src/bin/bfl_sales_bot.rs` | Rust bot with MySQL logging and A/B prompt variants |
| Credit Expert Bot (Python, legacy) | `credit_expert_bot.py` | Legacy bot pending migration |

## Dialog sources for testing
```
chats/
├── Хара.txt              # Primary channel ("Khara")
├── вайбкодеры.md         # Developer community ("vibecoders")
├── Hobbitkn.md           # Direct chat
├── Golang_GO.txt         # Golang chat
├── iriy5.txt             # Direct chat
└── ValTarobot.txt        # Bot chat
```
Export a new chat via Rust CLI:
```bash
cargo run -- tg "Chat Name" --limit 500
# or by alias from config.yml
./target/release/telegram_reader read dasha5 --limit 300
```

## Testing BFL Sales Bot (Rust)

### Run locally
```bash
cargo run --bin bfl_sales_bot
# or optimized
cargo build --release
./target/release/bfl_sales_bot
```
Required env vars: `BFL_SALES_BOT_TOKEN`, `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `OPENAI_API_KEY`, MySQL creds (`MYSQL_HOST`/`MYSQL_PORT`/`MYSQL_DATABASE`/`MYSQL_USER`/`MYSQL_PASSWORD`). Optional: `BFL_PROMPT_EXPERIMENT`, `OPENAI_MODEL`.

### Observability
```bash
RUST_LOG=info ./target/release/bfl_sales_bot
RUST_LOG=debug ./target/release/bfl_sales_bot  # verbose
```
If running as a service, use `journalctl -u bfl_sales_bot -f`.

### Telegram checks
1. Find the bot: `@BFL_sales_bot`
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
cargo test --bin bfl_sales_bot
```

### Prompt variants
Prompts are defined in `src/bin/bfl_sales_bot.rs` (`SALES_SYSTEM_PROMPT`, `FAST_CLOSE_PROMPT`, `STORY_PROOF_PROMPT`). A/B variants are assembled in `prompt_variants()`; adjust weights/temperature or text there. Experiments use `BFL_PROMPT_EXPERIMENT` (default `bfl_prompt_ab`) and are stored in MySQL `bot_experiments`.

## Testing Credit Expert Bot (Python, legacy)

### Run
```bash
uv run python credit_expert_bot.py
```
Env vars: `CREDIT_EXPERT_BOT_TOKEN`, MySQL credentials, `OPENAI_API_KEY`.

### Tests
```bash
uv run pytest tests/test_credit_expert_bot.py -v
uv run pytest tests/test_credit_expert_dialog.py -v
```

## MySQL database

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
SELECT * FROM bot_messages WHERE bot_name='BFL_sales_bot' ORDER BY created_at DESC LIMIT 20;

-- Sessions
SELECT * FROM bot_sessions WHERE is_active=TRUE;
```

### Review a dialogue from DB
```sql
SELECT direction, message_text, created_at
FROM bot_messages 
WHERE user_id = 5551302260 AND bot_name = 'BFL_sales_bot'
ORDER BY created_at DESC
LIMIT 50;
```

## Debugging

### Logs
```bash
RUST_LOG=debug ./target/release/bfl_sales_bot
uv run python credit_expert_bot.py 2>&1 | tee /tmp/credit_bot.log
```

### Send a test message via CLI
```bash
cargo run --bin send_message -- 5551302260 "Test message"
```

## Automated dialogue analysis
`test_bot_dialogue` is a Rust utility that scores bot conversations via OpenAI.

```bash
# From file
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file "@BFL_sales_bot.md"

# From MySQL by user_id
./target/release/test_bot_dialogue --bot @BFL_sales_bot --user-id 5551302260

# Interactive
./target/release/test_bot_dialogue --bot @BFL_sales_bot --interactive

# JSON for CI/CD
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file dialogue.md --json

# Only problems
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file dialogue.md --problems-only
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
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file dialogue.md
if [ $? -ne 0 ]; then
  echo "❌ Critical issues found"
  exit 1
fi
```
