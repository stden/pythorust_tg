# GitHub Actions Performance Playbook

This document records the CI/CD optimizations that actually work for this
repository. It is intentionally practical: every technique below either exists
in `.github/workflows/ci-cd.yml` or is a proven next step for this Rust/Python
codebase.

## The Root Cause That Made CI Green

Before any speed work, the pipeline was **red on every push** while everything
passed locally. Every Rust job failed in about one second:

```
error: could not execute process `sccache .../rustc -vV` (never executed)
       No such file or directory
```

The cause was a committed `.cargo/config.toml` containing:

```toml
[build]
rustc-wrapper = "sccache"
```

`sccache` exists on the developer machine but **not** on `ubuntu-latest`
runners. Cargo tried to wrap every `rustc` invocation with a missing binary, so
compilation never started.

The fix is to make the file local-only:

```bash
git rm --cached .cargo/config.toml
echo ".cargo/config.toml" >> .gitignore
```

It stays on dev machines (where `sccache` exists) and is simply absent on CI,
where Cargo falls back to plain `rustc`. This is the load-bearing change —
everything below is optimization on top of an already-green pipeline.

**Lesson:** never commit a `rustc-wrapper` (or any tool path) that is not
guaranteed to exist in every environment that reads the config.

## Current Pipeline Shape

The workflow is optimized around four facts:

- The repository has two large execution domains: Rust and Python.
- Most pull requests touch only one domain.
- Coverage and release artifacts are useful, but they are not useful on every
  PR commit.
- The slowest GitHub Actions cost is usually repeated dependency resolution,
  repeated Rust compilation, and redundant runs for old commits.

The current workflow uses:

- `changes` to classify changed files.
- `python` for Python linting and tests.
- `rust-fmt`, `rust-clippy`, and `rust-test` as parallel Rust jobs.
- `rust-coverage` only outside pull requests.
- `build-rust`, `build-python`, and `release` only for tags.
- `ci-status` as the stable required check for branch protection.

## What Actually Works

| Method | Why it works | Current implementation |
| --- | --- | --- |
| Cancel outdated PR runs | New commits make old PR runs obsolete. Canceling them saves queue time and runner minutes. | Workflow-level `concurrency` with `cancel-in-progress` for pull requests. |
| Path-filter Rust and Python jobs | A documentation-only or Python-only change should not compile the whole Rust workspace. | `dorny/paths-filter` emits `python`, `rust`, and `workflows` outputs. |
| Split independent jobs | `cargo fmt`, `cargo clippy`, and tests do not need to run sequentially. | Separate `rust-fmt`, `rust-clippy`, and `rust-test` jobs. |
| Cache dependencies and build outputs | Reusing Cargo registry/git data and Rust build artifacts avoids rebuilding the world. | `Swatinem/rust-cache@v2` with separate shared keys per Rust job type. |
| Use the mold linker | This crate links 53 binaries + a cdylib against heavy deps; linking, not codegen, is the bottleneck. mold links 2-5x faster than GNU `ld`. | `rui314/setup-mold` + `RUSTFLAGS=-Clink-arg=-fuse-ld=mold` on the debug Rust jobs. |
| Lockfiles not committed | Smaller repo; tools resolve from manifests each run. | `Cargo.lock` and `uv.lock` are both gitignored; CI passes no `--locked`/`--frozen`. Trade-off: builds are not version-pinned across runs. |
| Use uv cache and no-sync commands | Install once, then avoid repeated dependency checks for each command. | `astral-sh/setup-uv` cache plus `uv run --no-sync`. |
| Skip building the Python project during Python tests | Tests use pure-Python modules, not the compiled extension. Building the maturin project in the Python job is wasted work. | `uv sync --no-install-project --dev --no-progress`. |
| Use `cargo-nextest` for Rust tests | Nextest is faster than plain `cargo test` for ordinary Rust test binaries and gives better CI isolation. | `cargo nextest run --workspace --all-features`. |
| Run doctests separately | Nextest does not run doctests, so doctests need their own command. | `cargo test --workspace --doc --all-features`. |
| Disable debug info in CI builds | Debug symbols are expensive to generate and cache. Most CI compile/test jobs do not need them. | `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`. |
| Disable Cargo incremental compilation in CI | Incremental compilation can increase cache size and is less useful on fresh runners. | `CARGO_INCREMENTAL=0`. |
| Keep coverage out of PRs | Coverage instrumentation is slow and usually duplicates signal from tests. | `rust-coverage` and Python coverage run only outside pull requests. |
| Build release artifacts only for tags | Release builds are expensive and only useful for release events. | `build-rust`, `build-python`, and `release` run only on `refs/tags/*`. |
| Package Rust binaries dynamically | Manual binary lists drift and cause slow failures. Dynamic packaging also works as binaries are added or renamed. | `find target/release -maxdepth 1 -type f -executable`. |
| Add job timeouts | Hung CI should fail fast enough to unblock developers. | Every job has `timeout-minutes`. |
| Use least permissions by default | Security hardening does not speed CI directly, but it prevents expensive incident recovery and keeps release permissions explicit. | Workflow default `permissions: contents: read`; release job overrides to `contents: write`. |
| Keep dependency automation current | Old Actions and tool versions eventually become slow or broken. | Dependabot tracks GitHub Actions, Cargo, and pip ecosystems. |

## Workflow-Level Optimizations

### 1. Concurrency Cancellation

Use workflow-level concurrency for PRs:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

This works because only the latest commit on a pull request matters for review.
Do not use unconditional cancellation for release jobs if an older release run
must finish. The current expression cancels PR runs but does not cancel tag
release runs.

### 2. Path Filtering

The `changes` job prevents unrelated work:

```yaml
python:
  - "pyproject.toml"
  - "*.py"
  - "chat_analysis/**"
  - "integrations/**"
  - "python/**"
  - "tests/**/*.py"

rust:
  - "Cargo.toml"
  - "Cargo.lock"
  - "src/**"
  - "benches/**/*.rs"
  - "tests/**/*.rs"
```

This keeps the workflow itself running, but skips expensive jobs. That is safer
than trigger-level `paths` when branch protection expects a stable status check:
the lightweight `ci-status` job still reports a final result.

### 3. Stable Aggregate Status

Path-filtered jobs can be skipped. Branch protection is easier when it requires
one stable job:

```yaml
ci-status:
  if: always()
  needs: [changes, python, rust-fmt, rust-clippy, rust-test]
```

The job fails if any required job failed or was cancelled, and passes when jobs
passed or were intentionally skipped by filters.

## Python Optimizations

### 1. Use uv Cache

`astral-sh/setup-uv` supports built-in caching:

```yaml
- uses: astral-sh/setup-uv@v5
  with:
    enable-cache: true
    cache-dependency-glob: |
      pyproject.toml
```

This is better than hand-rolled pip cache steps because uv owns the dependency
layout.

### 2. Avoid Re-Syncing for Each Command

After dependencies are installed:

```bash
uv run --no-sync ruff check .
uv run --no-sync ruff format --check .
uv run --no-sync python -m pytest -q
```

`--no-sync` avoids repeated environment validation. It is safe when the workflow
has already run `uv sync`.

### 3. Skip Building the Project in Python Test Jobs

The Python CI job currently uses:

```bash
uv sync --no-install-project --dev --no-progress
```

This avoids building the maturin/PyO3 extension inside the Python lint/test job.
Keep this only while tests import pure-Python packages. If tests start requiring
the compiled `telegram_reader` extension, either build it explicitly in a
separate job or remove `--no-install-project`.

`uv.lock` is intentionally **not** committed (it is gitignored), so the Python
jobs do not use `--frozen`; `uv sync` resolves from `pyproject.toml` each run.
If you later want reproducible Python installs, commit `uv.lock` and add
`--frozen` back. Rust is treated the same way: `Cargo.lock` is gitignored and CI
passes no `--locked`; commit `Cargo.lock` and re-add `--locked` if you want
pinned, reproducible Rust builds.

### 4. Do Coverage Only When It Is Worth It

Pull requests need fast feedback. Pushes to `main` and tags can afford coverage:

```yaml
- name: Run fast tests
  if: github.event_name == 'pull_request'
  run: uv run --no-sync python -m pytest -q

- name: Run tests with coverage
  if: github.event_name != 'pull_request'
  run: uv run --no-sync python -m pytest --cov ...
```

This keeps PRs fast without dropping coverage entirely.

## Rust Optimizations

### 1. Split Rust Jobs

Do not run this sequence in one job:

```bash
cargo fmt
cargo clippy
cargo test
cargo tarpaulin
```

That makes the longest path equal to the sum of all work. Split the jobs so
formatting, linting, and tests can run in parallel. The workflow currently uses:

- `rust-fmt`
- `rust-clippy`
- `rust-test`
- `rust-coverage`

### 2. Use rust-cache Instead of a Raw target Cache

Use:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: rust-test
    cache-on-failure: true
```

This is usually better than manually caching `target/`, `~/.cargo/registry`,
and `~/.cargo/git` because `rust-cache` understands Cargo projects and prunes
irrelevant files.

Use separate shared keys for jobs with different compilation profiles:

- `rust-clippy`
- `rust-test`
- `rust-coverage`
- `rust-release`

Shared keys let related runs reuse artifacts without forcing every job to share
one giant, noisy cache.

### 3. Use nextest for Ordinary Tests

Run:

```bash
cargo nextest run --workspace --all-features
```

Nextest is designed for faster Rust test execution and better per-test process
isolation. It does not replace doctests, so keep:

```bash
cargo test --workspace --doc --all-features
```

### 4. Lockfiles Are Not Committed

`Cargo.lock` (like `uv.lock`) is gitignored, so CI does **not** pass `--locked`
— cargo resolves from `Cargo.toml` each run:

```bash
cargo clippy --workspace --all-targets --all-features -- ...
cargo nextest run --workspace --all-features
cargo build --workspace --all-features --release
```

If you want pinned, reproducible Rust builds, commit `Cargo.lock` and add
`--locked` back to these commands.

This prevents CI from silently resolving dependency versions differently from
the checked-in `Cargo.lock`.

### 5. Reduce Debug Info

These environment variables reduce Rust build artifact size:

```yaml
CARGO_PROFILE_DEV_DEBUG: "0"
CARGO_PROFILE_TEST_DEBUG: "0"
```

They are useful for CI compile/test jobs. If a job needs symbolized stack traces
or debugger artifacts, override the variable only for that job.

### 6. Keep Coverage Out of PRs

Rust coverage uses instrumentation and can be much slower than tests. The
workflow runs coverage only when `github.event_name != 'pull_request'`.

Use:

```bash
cargo tarpaulin --lib --engine llvm --out xml --skip-clean --fail-under 30
```

`--lib` is deliberate here: the repository has many thin CLI/bot binaries, and
library coverage gives the best signal-to-runtime ratio.

### 7. Use the mold Linker on Debug Jobs

This is the single biggest compile-speed win for this repository. The crate
links **53 `[[bin]]` targets + a PyO3 cdylib** against tokio, pyo3, teloxide,
kube, grammers, reqwest, and tonic. The link phase dominates the debug jobs,
and `mold` is a drop-in replacement for GNU `ld` that links 2-5x faster.

Apply it to the debug-build jobs (`rust-clippy`, `rust-test`, `rust-coverage`):

```yaml
env:
  RUSTFLAGS: "-Clink-arg=-fuse-ld=mold"
steps:
  - uses: dtolnay/rust-toolchain@stable
  - uses: rui314/setup-mold@v1
  - uses: Swatinem/rust-cache@v2
    with: { shared-key: rust-clippy, cache-on-failure: true }
```

Setting `RUSTFLAGS` at job level keeps mold scoped to Rust linking and lets
`rust-cache` key on it consistently. `ubuntu-latest` is Ubuntu 24.04 with GCC
13, which supports `-fuse-ld=mold` natively, so no extra linker driver is
needed. Verify the exact flag works against the real dependency graph before
trusting it:

```bash
RUSTFLAGS="-Clink-arg=-fuse-ld=mold" cargo build --lib   # → Finished
```

Do **not** add mold to the release (`build-rust`) job: thin-LTO release builds
produce few large objects where mold's symbol-resolution advantage is marginal,
and that job runs on tags only.

## Release Optimizations

### 1. Build Only on Tags

Release artifacts are created only for tag refs:

```yaml
if: startsWith(github.ref, 'refs/tags/')
```

This avoids release builds on ordinary pushes and pull requests.

### 2. Split Rust and Python Packaging

Rust and Python packaging are independent. The workflow builds them in separate
jobs:

- `build-rust`
- `build-python`

The final `release` job only downloads artifacts and publishes them.

Cache maturin's compiled cdylib across tag builds so repeated releases do not
recompile the PyO3 extension from scratch:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: maturin-build
    cache-on-failure: true
```

### 3. Package Rust Binaries Dynamically

Use a dynamic copy step:

```bash
find target/release -maxdepth 1 -type f -executable -print0 |
  xargs -0 -I{} cp {} artifacts/rust/
```

This avoids maintaining a manual list of dozens of binaries.

## What Does Not Usually Work

### One Giant Job

A single job with format, lint, test, coverage, and build is easy to read but
slow. It serializes independent work and wastes time after an early unrelated
failure.

### Running Coverage on Every Pull Request

Coverage is useful but slow. It should run on protected branches, scheduled
runs, or tags unless a project explicitly requires coverage gates on every PR.

### Manually Caching Everything

Caching all of `target/` manually often creates huge caches and stale artifacts.
Prefer `Swatinem/rust-cache` for Rust and setup-uv cache for Python.

### Building Release Artifacts Before Tests

Release builds are expensive. They should depend on test/lint jobs and only run
for release events.

### Trigger-Level Path Filters Without an Aggregate Check

`on.pull_request.paths` can skip the entire workflow. That is fast, but it can
be awkward with required checks because no status may be reported. Job-level
path filtering plus `ci-status` keeps branch protection predictable.

### Installing Cargo Tools from Source on Every Run

`cargo install cargo-tarpaulin` or `cargo install cargo-nextest` from source can
be very slow. Prefer prebuilt installers such as `taiki-e/install-action`.

### Ideas That Were Tested and Rejected

An automated audit proposed many optimizations. The following looked reasonable
but were checked against the real toolchain and found wrong or not worth it.
They are recorded here so nobody "rediscovers" them.

| Proposal | Verdict | Evidence |
| --- | --- | --- |
| Add `--no-progress` to cargo commands | Breaks the build | `cargo build --no-progress` → `error: unexpected argument '--no-progress'`. Cargo has no such flag (it was confused with uv's). |
| `cargo nextest run --lib --test '*'` to skip building bins in the test job | Weakens the gate | 6 binaries contain `#[cfg(test)]`/`#[test]` blocks (`community_game_bot`, `bulk_reactions`, `devops_ai_bot`, `task_assistant_bot`, `credit_expert_bot`, `send_viral_questions`). `--lib` would silently drop their unit tests. |
| Pin `runs-on: ubuntu-24.04` "for speed" | No-op | `ubuntu-latest` already resolves to 24.04. Zero speed delta; only adds manual-bump maintenance. |
| Re-enable `sccache` via `mozilla-actions/sccache-action` | Deferred | The action installs the binary first, so the original failure can't recur, but `rust-cache` + mold already cover the bottleneck. Re-introducing the exact component that broke CI is not worth doing without a live run to watch it go green. |
| `[profile.test] opt-level=1` | Deferred | Speeds test execution but can slow cold-cache compilation (the dominant CI cost). Net effect unproven here. |
| `pytest-xdist -n auto` | Deferred | `conftest.py` uses an autouse `clean_module_cache` fixture that can break under xdist workers, and coverage needs `coverage combine`. The small suite is already fast. |
| `cache-all-crates: true` | Deferred | Marginal gain but grows the cache toward the 10 GB limit, risking eviction churn. |

The principle: an audit is a list of *hypotheses*. Two of the "recommended"
items would have broken CI or dropped test coverage. Verify every change before
landing it.

## Measuring Whether an Optimization Works

Use real workflow data, not intuition.

Track these numbers before and after a change:

- PR wall-clock time from queued to completed.
- Total billable minutes.
- Slowest job duration.
- Cache hit/miss behavior.
- Time spent installing dependencies.
- Time spent compiling Rust crates.
- Time spent running coverage.

Useful commands:

```bash
gh run list --workflow "CI/CD" --limit 20
gh run view <run-id> --json jobs
```

In the GitHub UI, compare the same kind of run:

- PR with Rust-only changes.
- PR with Python-only changes.
- PR with docs-only changes.
- Tag release.

Do not compare a cold-cache release run with a warm-cache small PR. That will
produce misleading conclusions.

## Maintenance Checklist

When changing the workflow:

- Keep `ci-status` as the branch-protection target.
- Add new Rust paths to the `rust` filter.
- Add new Python paths to the `python` filter.
- Keep coverage out of PRs unless there is a strong policy reason.
- Keep release jobs tag-only.
- Do not pass `--locked` (Cargo.lock is gitignored); commit it first if you want pinning.
- Prefer prebuilt tool installers over compiling tools in CI.
- Run `actionlint` after editing workflow YAML.
- Parse workflow YAML locally before pushing.
- Keep Dependabot enabled for GitHub Actions, Cargo, and Python tooling.

## Commands Used to Validate Workflow Changes

Run these locally after editing workflow files:

```bash
actionlint
python3 - <<'PY'
from pathlib import Path
import yaml

for path in Path(".github").rglob("*.yml"):
    yaml.safe_load(path.read_text())
    print(f"{path}: ok")
PY
git diff --check -- .github
```

## References

- GitHub Actions workflow syntax:
  https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- GitHub Actions dependency caching:
  https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching
- GitHub Actions `GITHUB_TOKEN` permissions:
  https://docs.github.com/en/actions/tutorials/authenticate-with-github_token
- GitHub Actions events and path filtering:
  https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows
- `Swatinem/rust-cache`:
  https://github.com/Swatinem/rust-cache
- cargo-nextest GitHub Actions installation:
  https://nexte.st/docs/installation/pre-built-binaries/
