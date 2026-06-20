# AGENTS.md — Architecture & Reorganization Plan

This is the **living architecture / planning document** for the Telegram
Automation Toolkit. It is intentionally *not* a copy of the README.

- **Operator quickstart & usage** → [README.md](README.md) and [docs/](docs/)
- **Engineering rules (Rust-first, TDD, no `.unwrap()`)** → [CLAUDE.md](CLAUDE.md)
- **This file** → the current architecture map, the reorganization plan, and
  the status log of structural changes.

> Audit date: 2026-06-20. The plan below was produced from a 4-dimension
> repository audit (Rust/Cargo, Python legacy, docs/root clutter,
> build/CI/test/data) with an adversarial verification pass on every
> non-trivial move.

---

## 1. Architecture snapshot (current)

```
telegram_reader (Rust crate: lib + cdylib)
├── src/lib.rs            library root; re-exports config/error/integrations/prompts/session
├── src/main.rs           main CLI (clap derive) → dispatches to src/commands/*
├── src/commands/         CLI command logic (29 modules) — the real work lives here
├── src/bin/<category>/   53 thin binaries grouped by domain:
│     chat/ data/ export/ moderation/ analysis/ bots/ ops/ linear/ dev/
├── src/integrations/     AI clients: openai, gemini, claude, ollama, whisper, yandex_tts
├── src/analysis/         vector DB (Qdrant), graph DB (Neo4j), embeddings, models
├── src/analytics/        A/B testing, bot analytics, dialog evaluation
├── src/lightrag/         LightRAG: chunker, entity_extractor, graph, retriever
├── src/n8n/              n8n monitor + backup
├── src/{config,error,prompts,session,metrics,reactions,export,chat,linear}.rs
└── src/python.rs         PyO3 bindings (maturin module `telegram_reader`)

Python (legacy-only, migrated to Rust over time)
├── integrations/         AI client wrappers — ACTIVELY TESTED (pytest/behave)
├── chat_analysis/        legacy analyzer (analyzer/fetcher/llm_*/models/...)
├── python/telegram_reader/   maturin/PyO3 package shim
├── mcp_telegram_server.py    MCP server (⚠ currently broken — see §2)
└── (root scripts, mostly already migrated)

Meta
├── codev/                SPIDER-SOLO protocol, specs, plans, reviews
├── features/             behave BDD suites
├── tests/                Rust integration + Python pytest + Playwright (mixed)
├── benches/              Rust criterion (serialization, text_processing) + a stray .py
├── migrations/           one SQL file (schema is otherwise scattered)
└── .github/, .githooks/, scripts/
```

**Core pattern (keep it):** binaries in `src/bin/<category>/` are thin
wrappers; logic lives in `src/commands/` and the library modules. This is a
sound DRY layout — no change planned.

---

## 2. Audit findings (the "why")

### Confirmed structural problems
| # | Area | Problem | Severity |
|---|------|---------|----------|
| ~~F1~~ | Root | ~~Empty stub dirs at root~~ — **void**: `analysis/ analytics/ lightrag/ n8n/` are `src/` subdirs, not root dirs (an earlier `ls 2>/dev/null` swallowed the "no such dir" error). No action. | n/a |
| F2 | Cargo | `[[bin]]` entries fragmented across the file, unsorted, with `[workspace]` and `[[bench]]` interleaved **inside** the bin list; trailing "previously auto-discovered" block | med |
| F3 | Python | 4 root scripts fully migrated to Rust, kept as duplicates: `check_all_chats_tasks.py`, `collect_chat_ideas.py`, `load_messages_to_db.py`, `sync_linear_tasks.py` | high (DRY) |
| F4 | Python | Sanitization stripped 3 helper modules (`telegram_session`, `chat_export_utils`, `linear_client`) → `mcp_telegram_server.py`, `load_messages_to_db.py`, `sync_linear_tasks.py` import non-existent modules and **cannot run** | high |
| F5 | Python | `chat_analysis/llm_analyzer_refactored.py` is an unused duplicate of `llm_analyzer.py` (only the latter is imported/tested) | med |
| F6 | Docs | AGENTS.md ~90% duplicated README; config split across `ENV_SETUP.md` + `CONFIGURATION_SUMMARY.md`; guides (`CODEX.md`, `OPS_TOOLS.md`) and ops scripts/`PyO3.ru.typ` sit at root | med |
| F7 | Data | `bot_users` is INSERT-ed by `sales_bot`/`community_game_bot`/`credit_expert_bot` but **never created** (no `CREATE TABLE bot_users` anywhere); other bot DDL lives only inside `ensure_tables()` in Rust + duplicated in `ab_testing.py` | high (latent bug) |
| F8 | Tests | `tests/` mixes Rust, Python, and Playwright artifacts flatly; behave suites live in top-level `features/` | low-med |
| F9 | Build | **No committed `Cargo.lock`** (it was `.gitignore`d) on a binary/app crate → clean builds are non-reproducible and currently **fail**: a yanked transitive dep (`core2 0.4.0` via `grammers-crypto`→`glass_pumpkin`) can't be selected without a lock. Same failure blocks the maturin/PyO3 build, so `pytest` can't import `chat_analysis`. | high |

### False positives caught during verification (no action)
- "serialization bench not registered" — it **is** auto-discovered (`autobins=false` doesn't disable bench discovery). Adding `[[bench]]` would duplicate it.
- "Rust imports `ab_testing.py`" — impossible; Rust cannot import Python. `ab_testing.py` only *consumes* the PyO3 binding.

---

## 3. Reorganization plan

### Phase 1 — safe, reversible, zero behavior change *(execute now)*
- ~~**P1-a (F1)** Remove empty stub dirs.~~ Void — see F1.
- **P1-b (F2)** Rewrite `Cargo.toml`: all 53 `[[bin]]` grouped by category &
  sorted; `[workspace]`/`[[bench]]` moved to coherent positions. Bin
  names/paths byte-identical — verified with `cargo metadata`.
- **P1-c (F6)** Consolidate docs under `docs/`:
  - `CODEX.md` → `docs/testing.md`
  - `OPS_TOOLS.md` → `docs/operations.md`
  - `ENV_SETUP.md` + `CONFIGURATION_SUMMARY.md` → `docs/configuration.md` (merged)
  - `PyO3.ru.typ` → `codev/resources/pyo3-presentation.ru.typ`
  - ops templates `n8n_backup_cron.sh`, `n8n_monitor.service`, `create_venv.cmd`
    → `scripts/ops/`
  - **Keep at root** (tooling/convention): `README.md`, `CLAUDE.md`, `AGENTS.md`,
    `.env.example`, `.gitignore`, `Cargo.toml`, `pyproject.toml`, `run.sh`,
    `config.yml`, and **`devops_bot.yml`** (runtime config the `devops_ai_bot`
    binary loads by default from cwd — moving it would break the default path).
- **P1-d (F6)** Repurpose AGENTS.md (this file) as the architecture/plan doc
  and update every cross-reference: README "Documentation" index, the
  `OPS_TOOLS`/`ENV_SETUP` links in `codev/specs/*`, README line describing
  AGENTS.md.

### Phase 2 — deletions *(user-approved, DONE)*
- **P2-a (F3,F4)** ✅ Deleted the 4 migrated+broken root scripts. The CI `*.py`
  glob doesn't name them, so deletion alone is correct (no filter edit — narrowing
  the glob would risk under-triggering).
- **P2-b (F5)** ✅ Deleted `chat_analysis/llm_analyzer_refactored.py` (no importer).
- **P2-c** ⏸️ `mcp_telegram_server.py` kept as-is (decision: leave pending the
  specced Rust MCP server). `ab_testing.py` / `voice_utils.py` also kept.

### Phase 3 — behavior-changing
- **P3-a (F7)** ✅ DONE — `migrations/002_create_bot_tables.sql` is now the single
  source of truth (bot_users + sessions + messages + experiments) and `bot_users`
  was added to `sales_bot::ensure_tables()` (fixes the crash for the
  self-bootstrapping bot). `community_game_bot`/`credit_expert_bot` rely on the
  migration (they create no tables). README schema note updated.
  *Caveat:* couldn't run `cargo build/clippy/test` here (see F9); the edit passes
  `cargo fmt --check` and mirrors the adjacent `ensure_tables` blocks verbatim.
- **P3-b (F8)** ⏭️ Skipped by request (higher risk, modest gain).
- **P3-c** ⏭️ Binary-name cleanups (`export_chats_mysql` vs `export_chats_to_mysql`;
  `send_viral_question` vs `send_viral_questions`) — deferred; need confirmation
  they aren't distinct tools / baked into operator scripts.
- **F9 (build health)** ⚠️ `.gitignore` no longer ignores `Cargo.lock`. Remaining
  action (needs a working resolver): bump/pin the yanked dep chain, then
  `cargo generate-lockfile` and commit `Cargo.lock` to restore reproducible builds.

---

## 4. Status log
- [x] ~~P1-a empty stub dirs~~ — void (false finding; they are `src/` subdirs)
- [x] P1-b Cargo.toml reorganized — 53 bins, benches preserved, `cargo metadata` MATCH ✓
- [x] P1-c docs consolidated: `CODEX→docs/testing.md`, `OPS_TOOLS→docs/operations.md`,
  `ENV_SETUP`+`CONFIGURATION_SUMMARY→docs/configuration.md`,
  `PyO3.ru.typ→codev/resources/`, ops templates `→scripts/ops/`
- [x] P1-d cross-references updated: README index, `codev/specs/spec-2025-11-23-*`,
  `docs/operations.md` script paths, `.gitignore` dead negations removed
- [x] Phase 1 verification (within env limits): `cargo verify-project` ✓,
  `cargo metadata --no-deps` 53-bin MATCH ✓, `cargo fmt --check` ✓.
  Full `cargo build`/`clippy`/`test` blocked by F9 (yanked dep, not our changes).
- [x] P2-a/b deletions — 5 files removed; 57 pytest items still collect
  (the 4 `chat_analysis` collection errors are F9, pre-existing).
- [x] P3-a bot schema/migration + `bot_users` fix (fmt-checked; build blocked by F9)
- [x] P2-c / P3-b / P3-c — decided: keep mcp, skip test reorg, defer renames
- [ ] **Open (F9):** resolve yanked deps + commit `Cargo.lock`; then run the full
  `cargo build && cargo clippy --all-targets -- -D warnings && cargo test` gate.

---

## 5. Codev methodology
The project follows Codev (SPIDER-SOLO). Protocol: `codev/protocols/spider-solo/protocol.md`.
Specs in `codev/specs/`, plans in `codev/plans/`, reviews in `codev/reviews/`.
TDD agents (`tdd-tester`, `tdd-coder`, `tdd-refactorer`) live in `.claude/agents/`.
When a structural change here affects protocol docs, update them in the same pass.

## 6. Build health & known issues
**Reproducible builds are currently broken (F9) — highest-priority follow-up.**

- There is **no committed `Cargo.lock`** (it used to be `.gitignore`d). For a crate
  that ships ~53 binaries (an application), the lockfile **should** be committed.
- Without a lock, `cargo build` re-resolves from scratch and fails: a yanked
  transitive dependency — `core2 0.4.0` (and `glass_pumpkin 1.10.0`), pulled via
  `grammers-crypto 0.8` → `grammers-client 0.8` — has no selectable version.
- The same failure blocks the maturin/PyO3 wheel build, which is why
  `pytest` cannot import the `chat_analysis` package (4 collection errors).
- `.gitignore` no longer ignores `Cargo.lock`. To finish the fix (needs network +
  a working resolver, not available in this sandbox):
  1. Bump/relax the dependency that pulls the yanked crates (e.g. a newer
     `grammers-client`, or pin a non-yanked `glass_pumpkin`/`core2`).
  2. `cargo generate-lockfile` (or a successful `cargo build`).
  3. Commit `Cargo.lock`.
  4. Run the full gate: `cargo build && cargo clippy --all-targets -- -D warnings
     && cargo test`, and `uv run pytest -v`.

Everything else in Phases 1–3 was validated to the extent the sandbox allows
(`cargo verify-project`, `cargo metadata --no-deps` target-set match,
`cargo fmt --check`, pytest collection of the unaffected suite).
