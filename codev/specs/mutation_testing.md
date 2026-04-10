# Specification: Mutation Testing

## Tool
- **Name**: `cargo-mutants`
- **Purpose**: Identify gaps in test coverage by injecting synthetic bugs (mutants) and checking if tests fail.

## Scope
- **Target**: Rust workspace (`Cargo.toml`).
- **Exclusions**: 
  - `src/bin/*` (Binaries often have lower test coverage than libraries).
  - Tests themselves.

## Process
1.  Run `cargo mutants --list` to estimate the number of mutants.
2.  Run `cargo mutants --workspace` to execute tests against mutants.
3.  **Success Criteria**:
    - High "caught" ratio indicates strong test coverage.
    - "Missed" mutants indicate code that can be changed without breaking tests (potential coverage gaps).

## Output
- Console output summarizing caught/missed mutants.
- `mutants.out/` directory (ignored by git) containing detailed logs.
