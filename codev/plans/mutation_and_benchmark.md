# Plan: Mutation Testing and Benchmarking

## Objective
Enhance code reliability and performance visibility by executing mutation testing and running existing benchmarks.

## Steps

1.  **Define Specifications**
    *   Create `codev/specs/mutation_testing.md`: Define scope (Rust crates), tools (`cargo-mutants`), and success criteria.
    *   Create `codev/specs/benchmarking.md`: Define scope (text processing, HTTP benchmarks), tools (`criterion`, `http_bench`), and output format.

2.  **Mutation Testing (Rust)**
    *   Execute `cargo mutants` on the workspace.
    *   Focus on key libraries first (`src/lib.rs`, `src/linear.rs`) if the full run is too slow, otherwise run all.
    *   Analyze surviving mutants and report findings.
    *   *Note*: We will not automatically fix mutants, but report them for future refactoring.

3.  **Benchmarking (Rust)**
    *   Run standard criterion benchmarks: `cargo bench`.
    *   Run the HTTP load testing tool: `cargo run --release --bin http_bench -- --help` to verify it builds and runs (we won't DDOS a real target without permission, so we'll run a local test or dry run).

4.  **Git Operations**
    *   Commit and push changes after defining specs.
    *   Commit and push any artifacts or reports generated (if appropriate to check in).
