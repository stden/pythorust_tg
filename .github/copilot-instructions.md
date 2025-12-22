# GitHub Copilot Instructions

## Project
Telegram Automation Toolkit - Rust CLI with AI integrations.

## TDD Workflow

Follow **Red-Green-Blue** cycle:

1. **RED** - Write failing test first
2. **GREEN** - Minimum code to pass
3. **BLUE** - Refactor, tests stay green

## Test Format

### Rust
```rust
#[test]
fn test_<feature>_<scenario>() {
    // Arrange
    // Act
    // Assert
}
```

### Python
```python
def test_<feature>_<scenario>():
    # Arrange
    # Act
    # Assert
```

## Rust Guidelines

- Use `Result<T, E>` for errors, not `.unwrap()`
- Prefer iterators over loops
- Use `?` for error propagation
- `const` for compile-time constants
- References over cloning

## Naming

- `snake_case` - functions, variables
- `PascalCase` - types, traits
- `SCREAMING_SNAKE_CASE` - constants

## Before Commit

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Anti-patterns

- `unsafe` without docs
- `.unwrap()` in production
- Hardcoded paths
- Secrets in code
