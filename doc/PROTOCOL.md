<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Protocol

MCP over JSON-RPC 2.0, implemented by [`rmcp`](https://crates.io/crates/rmcp),
the official Rust SDK. This crate supplies the tools; the SDK supplies
the framing, the handshake, the sessions and the revisions.

## Transports

| Command | Transport | Endpoint | Revisions |
|---|---|---|---|
| `oxml-mcp` | stdio, one message per line | stdin and stdout | `2024-11-05` to `2026-07-28` |
| `oxml-mcp --transport streamable-http` | Streamable HTTP | `/mcp` on `127.0.0.1:8000` | `2025-11-25`, `2026-07-28` |
| `oxml-mcp --transport sse` | HTTP+SSE, the older transport | `/sse` and `/messages/` on `127.0.0.1:8000` | `2024-11-05` |

`--host` and `--port` move the HTTP listeners. Neither authenticates.

## Two current revisions, one endpoint

**`2025-11-25`** opens with `initialize`; the reply names the revision
the server agreed to and, over HTTP, carries an `Mcp-Session-Id` the
client sends back on every later request. `GET /mcp` with that header
opens the server-to-client event stream; `DELETE` ends the session.

**`2026-07-28`** has no handshake and no session. Each request carries
its revision and the client's capabilities in `params._meta`, under
`io.modelcontextprotocol/protocolVersion` and
`io.modelcontextprotocol/clientCapabilities`; `server/discover` returns
the revisions and capabilities the server has. Each request also
mirrors its method in an `Mcp-Method` header, and a tool call its
tool in `Mcp-Name`, so a gateway can route without reading the body
(SEP-2243); a header that is missing or disagrees with the body is
refused with `-32020`. Responses stream as server-sent events; `GET`
without a session is `405`.

Over stdio the same two lifecycles apply: the first line is either
`initialize` or a request carrying its revision in `_meta`. A first
line that is neither is refused and the session ends.

## Methods

| Method | Returns |
|---|---|
| `initialize` | Capabilities, negotiated revision, server name and version, instructions |
| `server/discover` | The same, for the stateless revision |
| `tools/list` | The four tools with input schema, output schema and annotations |
| `tools/call` | A tool result: text, and the same answer as `structuredContent` |
| `ping` | An empty result |

## Two kinds of failure

This is the part worth getting right, because a model only ever sees
one of them.

**A tool that ran and could not do the job** is a *successful*
JSON-RPC response carrying `isError: true`:

```json
{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"…not well-formed…"}],"isError":true}}
```

The model reads that text and can correct the document or tell the
user. Returning a transport-level error here would hide the failure
from the model entirely — the client would handle it, and the model
would be left waiting for a result that never arrives. A schema
violation is this kind of failure and also a complete validation
result, so it keeps its `structuredContent`. So is a call with a
required argument missing or mistyped: the SDK reports which field,
as text the model can act on.

**A request the protocol rejects** is a JSON-RPC error. The tool never
ran, so there is nothing for the model to learn from:

| Situation | Code |
|---|---|
| Unknown method | `-32601` |
| `id` present, `method` absent | a JSON-RPC error |
| Mirrored routing header missing or disagreeing with the body | `-32020` |

A tool the server does not have is *not* in this category, although
the SDK's default would put it there as `-32602`. The stateless HTTP
revision carries `-32602` as an HTTP 400, which a client reports as a
transport fault; the model that misspelt the name never sees why. It
is reported instead as an `isError` result naming the four tools that
exist.

## Bytes that are not JSON

Over HTTP the request is refused with `415`, which is how the SDK
reads a body that claims to be JSON and is not. Over stdio the SDK skips
the line and reads the next one, without a reply. Either way the
session continues: a server that exited on malformed input would be
trivially killable by anything able to write to it.

## Trying it by hand

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"me","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | oxml-mcp
```

Over HTTP, with a server started by `oxml-mcp --transport streamable-http`:

```bash
curl -s http://127.0.0.1:8000/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}'
```
