# Dependency upgrade and history consolidation

Goal: merge every current branch into `main`, upgrade all direct dependencies
to the latest stable registry releases, preserve existing behavior, then replace
the local `main` history with one root commit.

The implementation keeps the current Rust library/binary layout and Python
legacy interfaces. Compatibility changes belong at existing API call sites;
shared peer-reference conversion belongs in `src/chat.rs`. The final Git tree
must equal the tested, merged tree. Remote history is not rewritten.

- [x] Fetch current branch heads, create and verify a complete Git bundle, and
  preserve the original `main` under `refs/backups/`.
- [x] Merge all twelve Dependabot branches; retain both upgrades when adjacent
  dependency lines conflict.
- [x] Query crates.io, PyPI, and npm for stable versions. Update `Cargo.toml`,
  `pyproject.toml`, and `tests/playwright/package.json`.
- [x] Resolve `Cargo.lock`, `uv.lock`, and the Playwright package lock. Preserve
  `glass_pumpkin 2.0.0-rc0`: `grammers-crypto 0.10.0` does not compile against rc1.
- [x] Migrate grammers imports, session construction, peers, messages, and raw
  requests against the 0.10 source API; test peer-reference conversion offline.
- [x] Migrate OpenAI request imports, PyO3 extraction, rand selection, Kubernetes,
  database, and benchmark APIs where compilation exposes incompatible changes.
- [x] Validate Rust with `cargo fmt --check`, locked all-target checks, Clippy,
  tests, and build. Validate Python with frozen pytest and Playwright discovery
  plus runnable browser tests. Record pre-existing external-service limitations.
- [x] Verify every direct dependency against the registry audit, confirm all
  branch heads were merged, preserve the final merged commit under `refs/backups/`,
  create one root commit from its tree, and atomically update `refs/heads/main`.
- [x] Confirm one reachable main commit, identical tree, and clean worktree;
  report backup paths, verification results, and the new main commit.
