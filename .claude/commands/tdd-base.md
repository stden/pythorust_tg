# TDD Base

Rust-first repo: prefer Rust tests for all new functionality. Python tests are only for legacy scripts until they are migrated.

## Test Format

### Rust
```rust
#[test]
fn test_<feature>_<scenario>() {
    // Arrange - setup
    // Act - execute
    // Assert - verify
}
```

### Python
```python
def test_<feature>_<scenario>():
    # Arrange - setup
    # Act - execute
    # Assert - verify
```

## Commands
```bash
# Rust (recommended)
cargo test
cargo test test_name_substring -- --nocapture

# Python (legacy)
uv run pytest -v
uv run pytest -k test_name_substring -v
```
