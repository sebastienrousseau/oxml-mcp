#!/usr/bin/env bash
# Drive the real binary over stdio and assert its replies.
#
# These assert rather than print. A README full of example requests
# goes stale the moment behaviour changes and still looks correct;
# assertions fail CI instead.
set -euo pipefail

SERVER="${OXML_MCP:-oxml-mcp}"
FAILURES=0

# A session opens with a handshake: the client names the protocol
# revision it speaks and the server answers in one it supports. Every
# request in an example is sent after this exchange, and the reply
# asserted on is the last line the server writes.
INITIALIZE='{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"examples","version":"0"}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"notifications/initialized"}'

# call <description> <request-json> <substring the reply must contain>
call() {
  local description="$1" request="$2" want="$3"
  local reply
  reply="$(printf '%s\n' "$INITIALIZE" "$INITIALIZED" "$request" \
    | "$SERVER" 2>/dev/null | tail -n 1)"
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

# call_raw <description> <lines...> -- <substring the last reply must contain>
#
# For a session that is not the usual handshake followed by one
# request: the lines are sent exactly as given.
call_raw() {
  local description="$1"; shift
  local lines=()
  while [[ "$1" != "--" ]]; do
    lines+=("$1"); shift
  done
  local want="$2"
  local reply
  reply="$(printf '%s\n' "${lines[@]}" | "$SERVER" 2>/dev/null | tail -n 1)"
  if [[ "$reply" != *"$want"* ]]; then
    echo "FAIL: $description"
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
