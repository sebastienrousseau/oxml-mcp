#!/usr/bin/env bash
# Drive the real binary over stdio and assert its replies.
#
# These assert rather than print. A README full of example requests
# goes stale the moment behaviour changes and still looks correct;
# assertions fail CI instead.
set -euo pipefail

SERVER="${OXML_MCP:-oxml-mcp}"
FAILURES=0

# call <description> <request-json> <substring the reply must contain>
call() {
  local description="$1" request="$2" want="$3"
  local reply
  reply="$(printf '%s\n' "$request" | "$SERVER" 2>&1 | head -1)"
  if [[ "$reply" != *"$want"* ]]; then
    echo "FAIL: $description"
    echo "  request : $request"
    echo "  wanted  : $want"
    echo "  got     : $reply"
    FAILURES=$((FAILURES + 1))
  else
    echo "ok: $description"
  fi
}

finish() {
  if [[ "$FAILURES" -gt 0 ]]; then
    echo "$FAILURES assertion(s) failed"
    exit 1
  fi
  echo "all assertions passed"
}
