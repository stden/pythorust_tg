#!/usr/bin/env bash
# Enable the repo's git hooks (in .githooks/) via core.hooksPath.
#
# Safer than copying into .git/hooks/: the hooks live in the repo, are
# versioned with the code, and every clone gets them after one command.
#
# Usage:
#   ./scripts/install-git-hooks.sh        # enable
#   ./scripts/install-git-hooks.sh --off  # disable (back to .git/hooks/)
#   git commit --no-verify                # skip hooks for one commit
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[ -n "$ROOT" ] || { echo "not a git repository" >&2; exit 1; }
cd "$ROOT"

if [ "${1:-}" = "--off" ]; then
    git config --unset core.hooksPath || true
    echo "core.hooksPath cleared — git uses .git/hooks/ again"
    exit 0
fi

git config core.hooksPath .githooks
chmod +x .githooks/* 2>/dev/null || true
echo "✓ core.hooksPath = .githooks"
echo "  pre-commit will run: cargo fmt --all + ruff check --fix + ruff format"
echo "  skip once with: git commit --no-verify"
