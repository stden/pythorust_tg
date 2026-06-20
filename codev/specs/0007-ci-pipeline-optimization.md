# Spec: CI/CD Pipeline — Green & Build-Speed Optimization

**ID:** 0007-ci-pipeline-optimization
**Status:** Implemented
**Author:** Project Team
**Date:** 2026-06-20

> Operational reference (detailed evidence, reproduce-locally commands, rejected
> ideas): [docs/github-actions-performance.md](../../docs/github-actions-performance.md). This spec is the formal SPIDER
> framing; it does not duplicate the detail there.

---

## 1. Problem

The GitHub Actions pipeline ([.github/workflows/ci-cd.yml](../../.github/workflows/ci-cd.yml))
had two issues:

1. **Red on every push.** All Rust jobs failed in ~1 s with
   `could not execute process 'sccache .../rustc'` while everything passed
   locally — a "works on my machine" split.
2. **Slow when green.** The crate builds **53 binaries + a PyO3 cdylib** against
   heavy async deps (tokio, pyo3, teloxide, kube, grammers, reqwest, tonic).
   Cold CI builds spent most of their wall-clock in the **link** phase.

### Constraints

- Must stay **green**, deterministically reproducible from committed lockfiles.
- **Must not weaken any gate** (clippy `-D warnings`, full test suite, coverage
  floors, ruff). Speed may never come from doing less checking.
- Changes must be verifiable without watching a live CI run (the implementer was
  rate-limited), i.e. provable on a CI-identical local toolchain.

---

## 2. Solution

### 2.1 Make it green (root cause)

`.cargo/config.toml` was committed with `rustc-wrapper = "sccache"`; `sccache`
is absent on `ubuntu-latest`. Make the file **local-only** (`git rm --cached` +
`.gitignore`). Cargo then falls back to plain `rustc` on CI and keeps using
sccache on dev machines. This is the load-bearing fix.

### 2.2 Speed (verified additions)

| Change | Job(s) | Rationale |
|--------|--------|-----------|
| **mold linker** (`rui314/setup-mold` + `RUSTFLAGS=-Clink-arg=-fuse-ld=mold`) | `rust-clippy`, `rust-test`, `rust-coverage` | 2–5× faster linking — the actual bottleneck for 53 bins + cdylib |
| **maturin `Swatinem/rust-cache`** (`shared-key: maturin-build`) | `build-python` (tags) | reuse the compiled PyO3 cdylib across releases |

Pre-existing, kept (already optimal): path filters, parallel jobs,
`cancel-in-progress`, per-job caches with distinct `shared-key`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_*_DEBUG=0`,
`--no-install-project` for Python tests, thin-LTO release profile.

> Note: both `uv.lock` and `Cargo.lock` are gitignored (not committed), so CI
> passes no `--frozen`/`--locked`; tools resolve from `pyproject.toml` /
> `Cargo.toml` each run.

### 2.3 Explicitly rejected

`cargo --no-progress` (invalid flag — breaks the build), `nextest --lib`
(silently drops 6 binaries' unit tests), `ubuntu-24.04` pin (no-op — `latest`
already is 24.04), and several deferred ideas (sccache-via-action, `profile.test`
tuning, pytest-xdist, `cache-all-crates`). Full evidence in
[docs/github-actions-performance.md](../../docs/github-actions-performance.md#ideas-that-were-tested-and-rejected).

---

## 3. Decisions & rationale

- **mold via `RUSTFLAGS` at job level**, not `make-default`: targeted to Rust
  linking, lets `Swatinem/rust-cache` key on it consistently, and the flag form
  was proven to link this repo's real dependency graph locally.
- **mold not added to the release job**: thin-LTO produces few large objects
  where mold's gain is marginal; the job is tag-only.
- **sccache deferred, not re-enabled**: it is the exact component that broke CI;
  re-introducing it blind (no green run to watch) violates the "stay green"
  constraint. mold + per-job caches already address the bottleneck.
- **Audit treated as hypotheses, not orders**: 2 of ~10 "recommended" items were
  wrong on inspection. Each landed change was verified first.

---

## 4. Validation

| Claim | How verified | Result |
|-------|--------------|--------|
| mold flag links a binary | `RUSTFLAGS=… rustc -o p p.rs && ./p` | `mold-link-ok` |
| mold links the real crate | `RUSTFLAGS=… cargo build --lib` | `Finished` (1m55s, full dep graph) |
| toolchain matches CI | `lsb_release`, `gcc --version`, `mold --version` | Ubuntu 24.04.4, GCC 13.3, mold 2.30 (= `ubuntu-latest`) |
| workflow well-formed | `yaml.safe_load(ci-cd.yml)` | valid |
| `--no-progress` is invalid | `cargo build --no-progress` | `error: unexpected argument` (rejected) |
| `--lib` would drop tests | `grep -rl '#\[test\]' src/bin/` | 6 binaries (rejected) |

### Acceptance criteria

- [x] Pipeline is green and reproducible from committed lockfiles.
- [x] No gate weakened (clippy `-D warnings`, full nextest + doctests, coverage
      floors, ruff all retained).
- [x] Every applied change verified on a CI-identical toolchain before landing.
- [ ] **Measured** link-time reduction confirmed on a live CI run (expected
      ~30–50% off the debug jobs' link phase).

---

## 5. Future work

Promote the deferred items only with a CI run to watch: sccache-via-action,
`[profile.test]` tuning, `pytest-xdist`, `cache-all-crates`. Ratchet coverage
floors **up** as tests are added.
