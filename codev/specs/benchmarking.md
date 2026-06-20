# Specification: Benchmarking

## Tools
1.  **Criterion**: Standard Rust benchmarking framework.
2.  **`http_bench`**: Custom internal HTTP load testing binary.

## Scope

### 1. Text Processing (Criterion)
- **File**: `benches/text_processing.rs`
- **Scenarios**:
  - Tokenization speed.
  - Regex matching performance.
  - Large text payload handling.

### 2. HTTP Load Testing (`http_bench`)
- **File**: `src/bin/ops/http_bench.rs`
- **Capabilities**:
  - Concurrent requests.
  - Latency measurement (P50, P90, P99).
  - Request/sec throughput.

## Process
1.  **Criterion**: Execute `cargo bench`.
2.  **HTTP Bench**: Build release binary `cargo build --release --bin http_bench` and verify execution.

## Output
- Console output with statistical analysis.
- `target/criterion/` (HTML reports, ignored by git).
