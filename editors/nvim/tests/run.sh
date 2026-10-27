#!/usr/bin/env bash
# Run Neovim LSP integration tests headlessly
# Usage: ./tests/run.sh [test_name]
# Example: ./tests/run.sh diagnostics
#          ./tests/run.sh  (runs all tests)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$(dirname "$PLUGIN_DIR")")"

# Ensure the LSP binary is built
if [ ! -f "$REPO_ROOT/target/debug/typedown-lsp" ]; then
  echo "Building typedown-lsp..."
  cargo build -p typedown-server --manifest-path "$REPO_ROOT/Cargo.toml"
fi

TESTS=("$@")
if [ ${#TESTS[@]} -eq 0 ]; then
  TESTS=(diagnostics completion definition hover)
fi

EXIT_CODE=0

for test_name in "${TESTS[@]}"; do
  test_file="$SCRIPT_DIR/${test_name}/${test_name}.lua"
  if [ ! -f "$test_file" ]; then
    echo "SKIP: $test_file not found"
    continue
  fi

  echo "=== $test_name ==="
  if nvim --headless -u "$SCRIPT_DIR/minimal_init.lua" -l "$test_file" 2>&1; then
    echo ""
  else
    EXIT_CODE=1
    echo ""
  fi
done

exit $EXIT_CODE
