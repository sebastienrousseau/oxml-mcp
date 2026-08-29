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

# Found rather than listed. This loop named `session.sh errors.sh`,
# so an example added later was never run -- and these examples assert
# rather than print, which makes an unrun one a check that silently
# stopped checking.
#
# `lib.sh` is sourced by the others rather than run on its own, and
# running this script from inside its own loop would recurse. Both are
# skipped by name; everything else matching *.sh is an example.
status=0
found=0
for script in *.sh; do
  case "$script" in
    run-all.sh | lib.sh) continue ;;
  esac
  found=$((found + 1))
  echo
  echo "=== $script ==="
  bash "$script" || status=1
done

# A glob that matches nothing would otherwise report success over an
# empty run, which is the same failure in a different place.
if [[ $found -eq 0 ]]; then
  echo "no example scripts found in $(pwd)" >&2
  exit 1
fi

echo
echo "ran $found example scripts"
exit $status
