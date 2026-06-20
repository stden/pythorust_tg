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
├── src/bin/<category>/   52 specialized binaries grouped by domain:
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
├── migrations/           SQL migrations: viral questions + bot tables
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
| F9 | Build | ✅ **RESOLVED** — was: no committed `Cargo.lock` + a fully-yanked `core2` (via `grammers-crypto`→`glass_pumpkin`) made clean builds non-reproducible and failing. Fixed by vendoring `core2 0.4.0` + `[patch.crates-io]` + committing `Cargo.lock`; current verification and remaining full-gate follow-up are in §6. | resolved |

### False positives caught during verification (no action)
- "serialization bench not registered" — it **is** auto-discovered (`autobins=false` doesn't disable bench discovery). Adding `[[bench]]` would duplicate it.
- "Rust imports `ab_testing.py`" — impossible; Rust cannot import Python. `ab_testing.py` only *consumes* the PyO3 binding.

---

## 3. Reorganization plan

### Phase 1 — safe, reversible, zero behavior change *(execute now)*
- ~~**P1-a (F1)** Remove empty stub dirs.~~ Void — see F1.
- **P1-b (F2)** Rewrite `Cargo.toml`: all 53 Cargo bin targets (main CLI +
  52 specialized `src/bin/<category>/` entries) grouped by category & sorted;
  `[workspace]`/`[[bench]]` moved to coherent positions. Bin names/paths
  byte-identical — verified with `cargo metadata`.
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
  *Caveat:* at implementation time, the full Rust gate was still blocked by F9;
  the edit passed `cargo fmt --check` and mirrors the adjacent `ensure_tables`
  blocks verbatim. F9 has since been worked around with the committed lockfile
  and vendored `core2` patch.
- **P3-b (F8)** ⏭️ Skipped by request (higher risk, modest gain).
- **P3-c** ⏭️ Binary-name cleanups (`export_chats_mysql` vs `export_chats_to_mysql`;
  `send_viral_question` vs `send_viral_questions`) — deferred; need confirmation
  they aren't distinct tools / baked into operator scripts.
- **F9 (build health)** ✅ Resolved by committing `Cargo.lock` and adding the
  temporary `core2` path patch in `Cargo.toml` (`vendor/core2-0.4.0`). Remaining
  follow-up: keep the vendor patch until upstream drops the yanked dependency
  and add `--locked` to CI after the full Rust gate is green with the lockfile.

---

## 4. Status log
- [x] ~~P1-a empty stub dirs~~ — void (false finding; they are `src/` subdirs)
- [x] P1-b Cargo.toml reorganized — 53 Cargo bin targets (main CLI + 52 `src/bin`),
  benches preserved, `cargo metadata` MATCH ✓
- [x] P1-c docs consolidated: `CODEX→docs/testing.md`, `OPS_TOOLS→docs/operations.md`,
  `ENV_SETUP`+`CONFIGURATION_SUMMARY→docs/configuration.md`,
  `PyO3.ru.typ→codev/resources/`, ops templates `→scripts/ops/`
- [x] P1-d cross-references updated: README index, `codev/specs/spec-2025-11-23-*`,
  `docs/operations.md` script paths, `.gitignore` dead negations removed
- [x] Phase 1 verification (within env limits): `cargo verify-project` ✓,
  `cargo metadata --no-deps` 53-bin MATCH ✓, `cargo fmt --check` ✓.
  Full `cargo build`/`clippy`/`test` was blocked by F9 at the time; F9 is now
  worked around with the committed lockfile and vendored `core2` patch.
- [x] P2-a/b deletions — 5 files removed; pytest now passes after the F9
  lockfile/vendor workaround.
- [x] P3-a bot schema/migration + `bot_users` fix (fmt-checked at the time;
  Rust/Python checks now pass after the F9 workaround)
- [x] P2-c / P3-b / P3-c — decided: keep mcp, skip test reorg, defer renames
- [x] **F9 RESOLVED:** vendored `core2 0.4.0` + `[patch.crates-io]` + committed
  `Cargo.lock`; full `cargo build` was documented as verified (exit 0, all 53
  bins). Fresh docs-pass verification: `cargo check --workspace --all-features`
  and `uv run pytest -q` pass. Remaining gate: `clippy -D warnings` and
  `cargo test`.

---

## 5. Codev methodology
The project follows Codev (SPIDER-SOLO). Protocol: `codev/protocols/spider-solo/protocol.md`.
Specs in `codev/specs/`, plans in `codev/plans/`, reviews in `codev/reviews/`.
TDD agents (`tdd-tester`, `tdd-coder`, `tdd-refactorer`) live in `.claude/agents/`.
When a structural change here affects protocol docs, update them in the same pass.

## 6. Build health & known issues
**F9 RESOLVED (2026-06): the Rust build is reproducible and green again.**

- **Root cause:** `core2` is yanked in *every* published version on crates.io,
  yet the whole `glass_pumpkin` 1.x line (pulled by `grammers-crypto`, used by
  `grammers-client` 0.8 **and** 0.9) depends on `core2 ^0.4`. With no committed
  `Cargo.lock`, a fresh resolve could not select the yanked crate, so
  `cargo build`/`generate-lockfile` failed — and the same failure blocked the
  maturin/PyO3 wheel, so `pytest` couldn't import `chat_analysis`.
- **Fix:** the still-downloadable `core2 0.4.0` source is vendored under
  `vendor/core2-0.4.0/` and patched in via `[patch.crates-io]` in `Cargo.toml`;
  `Cargo.lock` is now committed (the `.gitignore` rule was removed — this is a
  binary/app crate, so the lockfile belongs in git).
- **Verified:** `cargo generate-lockfile` resolves (core2 0.4.0 pinned via the
  vendored patch, glass_pumpkin 1.9.0 selected). A full `cargo build` is
  documented as previously verified; fresh docs-pass verification on 2026-06-28:
  `cargo check --workspace --all-features` and `uv run pytest -q` both pass.
- **Cleanup later:** drop `vendor/core2-0.4.0/` + the `[patch.crates-io]` entry
  once upstream un-yanks `core2` or `grammers-crypto` drops `glass_pumpkin`.

## 7. Dependency upgrade log (2026-06)
- **Python (pyproject.toml):** all floors bumped to PyPI latest. The
  dependabot pip PRs (#29–#33: openai, pytest, pytest-cov, python-dotenv, ruff)
  were merged on GitHub; the 2 GitHub-Actions bumps (setup-python 5→6,
  setup-uv 5→7) were applied locally and pushed. ✅
- **Rust (Cargo.toml):** semver-**compatible** bumps applied — tokio 1.52,
  clap 4.6, bytes 1.12, qdrant-client 1.18, uuid 1.23, kube 3.1, tempfile 3.27. ✅
- **Rust breaking majors — DEFERRED** (need code migration + a buildable tree):
  grammers 0.9, async-openai 0.32→0.41, pyo3 0.24→0.29, kube 3→4, rand 0.8→0.10,
  schemars 0.8→1.2, reqwest 0.12→0.13, criterion 0.5→0.8, mysql_async 0.37,
  k8s-openapi 0.28. Do these on a branch with `cargo build`/`clippy`/`test`
  once F9 is resolved.

Everything validated to the extent this docs pass required: `cargo metadata --no-deps`
53-bin target-set match (main CLI + 52 `src/bin`), `cargo generate-lockfile`,
`cargo check --workspace --all-features`, `uv run pytest -q`, and `git diff --check`.
