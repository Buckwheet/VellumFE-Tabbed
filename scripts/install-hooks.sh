#!/usr/bin/env bash
# Install git hooks from .githooks/ into .git/hooks/
set -e
HOOKS_DIR="$(git rev-parse --show-toplevel)/.githooks"
GIT_HOOKS="$(git rev-parse --show-toplevel)/.git/hooks"

for hook in "$HOOKS_DIR"/*; do
    name="$(basename "$hook")"
    cp "$hook" "$GIT_HOOKS/$name"
    chmod +x "$GIT_HOOKS/$name"
    echo "Installed $name"
done
echo "Done. Git hooks installed."
