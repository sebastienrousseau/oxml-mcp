<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# 0001. Serve stdio, streamable HTTP and SSE from one command line

- **Status:** Accepted
- **Date:** 2026-09-19
- **Deciders:** maintainer

## Context

Until 0.0.8 the server was a hand-written JSON-RPC loop over stdio,
speaking MCP `2024-11-05` and nothing else. A client spawned the
process; nothing listened. That fits a developer's laptop. It does not
fit a shared deployment, a gateway that fans one server out to many
agents, or an auditor such as scout that speaks HTTP. The MCP
specification now has two current revisions on the HTTP binding,
`2025-11-25` (an `initialize` handshake and a session header) and
`2026-07-28` (stateless, per-request `_meta`, `server/discover`), and
clients on either must be served. The older HTTP+SSE transport
(`2024-11-05`) is still what some hosts expect.

Keeping up with three revisions by hand is a protocol project, not an
XML one. The official Rust SDK, `rmcp`, implements all of them, is
maintained by the people who write the specification, and is what the
sibling Rust servers of the suite are moving to.

## Options considered

1. Stay on stdio and the hand-written loop, and leave remote use to a
   wrapper process.
2. Extend the hand-written loop with HTTP, sessions and both current
   revisions.
3. Adopt `rmcp` for the protocol, keep the four tools as plain
   functions, and put the transport dispatch in one module copied
   verbatim into every Rust server of the suite.

## Decision

Option 3. `oxml-mcp` runs stdio; `oxml-mcp --transport
streamable-http` listens on `--host`/`--port` at `/mcp` and speaks both
current protocol revisions on that one endpoint, streaming responses as
server-sent events and offering the server-to-client stream on `GET`;
`oxml-mcp --transport sse` serves the older HTTP+SSE transport at
`/sse` and `/messages/`. The SDK has no server side for that last
transport, so `src/transport.rs` carries a small bridge: one event
stream per session, a `POST` endpoint that feeds it, and the SDK's
service running over an in-memory pair of channels.

The module binds loopback unless told otherwise and adds no
authentication of its own: a routable deployment sits behind a gateway
the operator trusts. It depends only on the SDK and on a
`ServerHandler` passed in, so the same file serves `rlg-mcp` and
`noyalib-mcp` unchanged.

The tools keep their names, argument names and descriptions. Each now
also declares an `outputSchema` and returns its answer as structured
content beside the text, and carries read-only annotations. A tool
that ran and failed is still an `isError` result the model can read; a
request the protocol rejects is still a JSON-RPC error.

## Consequences

`main()` delegates to `transport::run`, which is the same file in
every server, so the suite is started, documented and tested the same
way. The server is verified over streamable HTTP with scout in both
protocol eras and over SSE with the SDK client before release.

The crate is no longer dependency-free: the SDK brings `tokio`,
`axum` and `serde`. The security model's "no code that opens a socket"
claim is replaced by a narrower one: the tools never open a file or a
socket, and the listeners exist only when asked for on the command
line. The hand-written loop's `handle_line` and `serve` are gone; the
library's public surface is the four functions and the handler.

A stdio client must now perform the handshake -- or, in the stateless
revision, name its protocol version in `_meta` -- before its first
request. A bare `tools/call` as the first line, which the old loop
tolerated, is refused.
