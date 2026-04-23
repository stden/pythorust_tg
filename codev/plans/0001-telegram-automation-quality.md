# CODEV Plan: Telegram Reader/Auto-responder Reliability

> Protocol: SPIDER-SOLO  
> Linked spec: `codev/specs/0001-telegram-automation-quality.md` (TBD)  
> Context: Rust utilities `telegram_reader` (export, auto-responder, Linear CLI), Playwright test `example-site`

## Goals
- Improve reliability of export/auto-responder and cover key paths with tests
- Harden session/config handling to avoid races and leaks
- Improve observability and CLI UX

## Iteration 2025-11-24 (security)
- `config.yml` sanitized (placeholders + env only), removed real `api_id/api_hash/phone/user` and chats.
- OpenAI key now only read from env in `autoanswer.py` (no YAML fallback).
- Session-missing message no longer prints phone number (`telegram_session.py`) to reduce leakage risk.
- Follow-ups: review chat logs (e.g., `vibecod3rs.md`) and move to private storage if needed; rotate keys if old values may have leaked.
- Tests not run in this iteration.

## Phases

**Phase 1: Baseline protection and tests**
- Objective: Close gaps in unit tests and stabilize existing ones (Rust, Playwright).
- Tasks:
  - Add unit tests for `reactions`, `export`, `linear` (edge cases for empty input, trimming, HTTP/JSON errors).
  - Run Playwright `tests/playwright/tests/example-site.spec.ts`, fix failures.
  - Make `cargo test` part of CI (or local checklist) before push.
- Deliverable: green `cargo test` and up-to-date UI tests.
- Definition of done: `cargo test`; `npx playwright test tests/playwright/tests/example-site.spec.ts`; Playwright bugs fixed or documented.
- Risks/mitigation: Flaky UI tests → tighten locators/timeouts, allow `--headed`/tracing for debugging.

**Phase 2: Sessions and configuration**
- Objective: Prevent session access conflicts and config errors.
- Tasks:
  - Validate env/credentials at CLI/bin startup; friendly errors.
  - Ensure `SessionLock` removes the lock file on panic/Drop; add test for double acquisition.
  - Document `init_session` steps (README or HOWTO).
- Deliverable: updated init/lock module and brief how-to.
- Definition of done: unit tests for lock/cleanup; manual `cargo run --bin init_session`; lock file removed after forced drop.
- Risks/mitigation: Inconsistent lock on panic → test with staged panic + cleanup; Windows path differences → account for lock path portability.

**Phase 3: Export and media**
- Objective: Guarantee correct metadata and file layout when exporting.
- Tasks:
  - Normalize string format (timestamps, emoji), trim whitespace.
  - Create media directories before writing; cover with a test.
  - Handle missing sender/peer (fallback names).
  - Add an integration scenario with fixture messages (markdown + media paths).
- Deliverable: stable export (Markdown + media directories).
- Definition of done: unit/integration tests for `ExportWriter`, verify real markdown output; media dirs created/cleaned correctly.
- Risks/mitigation: Format changes might break Python scripts → document format in README; potential media write races → optional file lock or sequential writes.

**Phase 4: Auto-responder and observability**
- Objective: Make auto-responder resilient and transparent.
- Tasks:
  - Structured logging (tracing) for key events.
  - Timeouts/retries for OpenAI requests, error classes (network/limits).
  - Metrics for successful replies and session failures; protect session from crashing on 429/5xx.
- Deliverable: auto-responder that recovers from API errors and logs clearly.
- Definition of done: unit tests for error handling; local run with fake OpenAI response + simulated 429/5xx; logs include correlation-id and statuses.
- Risks/mitigation: OpenAI rate limits → backoff + jitter; session leak on panic → guard wrappers around network calls.

**Phase 5: CLI/Linear UX**
- Objective: Make CLI commands predictable and reduce user error.
- Tasks:
  - Validate `linear` args (keys/command/fields) + friendly hints for empty titles/commands.
  - Cache team-id and enforce strict HTTP/JSON error handling; tests for trimming input.
  - Update CLI help/README for new checks.
- Deliverable: updated CLI with clear messaging.
- Definition of done: `cargo test -p telegram_reader linear::tests`; manual `cargo run --bin linear ...` with test server; help reflects validations.
- Risks/mitigation: Breaking existing scripts → keep compatible flags/aliases, add warnings instead of hard errors when possible.

## Open questions
- Single `.env` for Rust/Playwright or separate?
- Should media download run in CI or be skipped (as in Python scripts)?
- Do we need integration tests against a Telegram "sandbox" or just mocks?

## Artifacts and control
- Plan: `codev/plans/0001-telegram-automation-quality.md` (this doc).
- Spec: `codev/specs/0001-telegram-automation-quality.md` (create after clarifying open questions).
- Review: `codev/reviews/0001-telegram-automation-quality.md` (fill after completion).
- Run checklists: `cargo test`, `npx playwright test tests/playwright/tests/example-site.spec.ts`, manual `cargo run --bin init_session` and `cargo run --bin linear ...`.
