#!/usr/bin/env bash
# The directory manifests must name the version Cargo.toml ships.
#
# glama.json is what Glama shows and server.json is what the MCP registry
# publishes from a tag. A manifest that lags the crate advertises an
# install command for an image that is not the current release.
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

want=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
status=0
check() {
  if [ "$2" != "$want" ]; then echo "MISMATCH $1: $2 (crate is $want)"; status=1; else echo "ok $1: $2"; fi
}
check "glama.json version"        "$(jq -r .version glama.json)"
check "glama.json docker tag"     "$(jq -r '.installation.docker' glama.json | sed 's/.*://')"
check "glama.json mcpServers tag" "$(jq -r '.mcpServers.oxml.args[-1]' glama.json | sed 's/.*://')"
check "server.json version"       "$(jq -r .version server.json)"
check "server.json image tag"     "$(jq -r '.packages[0].identifier' server.json | sed 's/.*://')"
check "CHANGELOG latest heading"  "$(sed -n 's/^## \[\([0-9][^]]*\)\].*/\1/p' CHANGELOG.md | head -1)"
exit $status
