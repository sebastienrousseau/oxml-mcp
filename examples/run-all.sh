#!/usr/bin/env bash
# Run every example against a freshly built binary.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

if [[ -z "${OXML_MCP:-}" ]]; then
  (cd .. && cargo build --release --quiet)
  OXML_MCP="${CARGO_TARGET_DIR:-$(cd .. && pwd)/target}/release/oxml-mcp"
  export OXML_MCP
fi
echo "using $OXML_MCP"

status=0
for script in *.sh; do
  # Skip run-all.sh so the runner does not recursively execute itself.
  if [[ "$script" == "run-all.sh" ]]; then
    continue
  fi
  # Skip sourced helpers (e.g. prefixed with '_' or containing 'helper'/'common')
  # as they are meant to be sourced by examples rather than executed as standalone tests.
  if [[ "$script" == _* || "$script" == "common.sh" || "$script" == *"helper"* ]]; then
    continue
  fi

  echo
  echo "=== $script ==="
  bash "$script" || status=1
done
exit $status