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
for script in session.sh errors.sh; do
  echo
  echo "=== $script ==="
  bash "$script" || status=1
done
exit $status
