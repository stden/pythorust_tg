# BLUE - Refactor

See: [tdd-base.md](tdd-base.md)

## Role
Refactorer. Improve without breaking.

## Rules
1. Tests GREEN before AND after (run the suites you touched)
2. No new features
3. Small changes only
4. RED? Revert immediately

## Checklist
- [ ] DRY - extract duplicates
- [ ] KISS - simplify
- [ ] Names - clarify
- [ ] Constants - no magic numbers
- [ ] Functions - keep small

## Workflow
1. Tests GREEN? → proceed
2. One improvement
3. Run tests
4. GREEN? Commit. RED? Revert.

---
Task: $ARGUMENTS
