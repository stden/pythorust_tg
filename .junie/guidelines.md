# Junie Guidelines - Telegram Automation Toolkit

## Project Overview
Rust CLI for Telegram automation with AI integrations.

## Tech Stack
- **Language**: Rust (primary), Python (legacy)
- **Async Runtime**: Tokio
- **Telegram**: Grammers (MTProto)
- **AI**: OpenAI, Gemini, Claude, Ollama
- **CLI**: Clap

## TDD Workflow

This project follows strict **Test-Driven Development** with Red-Green-Blue cycle.

### Phase 1: RED (Write Failing Test)
Before implementing ANY feature:
1. Write a test that describes expected behavior
2. Test MUST fail (proves it tests the right thing)
3. Use descriptive test names: `test_<feature>_<scenario>`

### Phase 2: GREEN (Make It Pass)
Write minimum code to pass:
1. Simplest possible implementation
2. Hardcoding is OK initially
3. No optimization, no refactoring
4. Just make tests GREEN

### Phase 3: BLUE (Refactor)
Improve code quality:
1. Tests must stay GREEN
2. Small incremental changes
3. Run tests after each change
4. Revert if tests fail

## Rust Coding Standards

### Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_scenario() {
        // Arrange
        let input = ...;

        // Act
        let result = function(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Error Handling
- Use `Result<T, E>` for fallible operations
- Use `?` operator for propagation
- Custom error types with `thiserror`
- NO `.unwrap()` in production code

### Performance
- Use iterators over loops
- Avoid unnecessary allocations
- Prefer references over cloning
- Use `&str` over `String` where possible

### Naming
- `snake_case` for functions and variables
- `PascalCase` for types and traits
- `SCREAMING_SNAKE_CASE` for constants
- Descriptive names over comments

## Commands

### Before Commit
```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

### Build
```bash
cargo build --release
```

## File Structure
```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Public API
├── commands/         # CLI commands
├── integrations/     # AI providers
└── analysis/         # Data analysis
```

## Anti-patterns to Avoid
- `unsafe` without documentation
- `.unwrap()` in non-test code
- Hardcoded paths
- Secrets in code
- Blocking async code
