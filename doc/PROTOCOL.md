<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Protocol

JSON-RPC 2.0 over stdin and stdout, one message per line. MCP protocol
version `2024-11-05`.

## Methods

| Method | Returns |
|---|---|
| `initialize` | Capabilities, protocol version, server name and version |
| `tools/list` | The four tools with their JSON schemas |
| `tools/call` | A tool result |

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
would be left waiting for a result that never arrives.

**A request the protocol rejects** is a JSON-RPC error. The tool never
ran, so there is nothing for the model to learn from:

| Situation | Code |
|---|---|
| Malformed JSON | `-32700` |
| `id` present, `method` absent | `-32600` |
| Unknown method | `-32601` |
| Unknown tool, invalid arguments | `-32602` |

MCP places unknown tools and invalid arguments in this category
deliberately: they are the client's mistake, not the model's.

## A request with an id and no method

Returns `-32600`. This was once answered with nothing at all, which
left a client waiting on a response that would never come — worse than
an error, because it looks like a hang.

## The server does not exit on bad input

A parse error is answered and the next line is read. A server that
exited on malformed JSON would be trivially killable by anything that
could write to its stdin.

## Trying it by hand

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | oxml-mcp
```
