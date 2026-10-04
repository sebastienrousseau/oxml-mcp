<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Security model

Two hostile inputs, not one.

**The document**, because a model was asked to look at something from
the internet. **The JSON around it**, because that is the program's
entire input and it arrives from a process this one does not control
-- on stdin, or over an HTTP listener that does not authenticate.

## The document

- **External entities are never dereferenced.** A document containing
  `<!ENTITY xxe SYSTEM "file:///etc/passwd">` cannot make the server
  read that file. The tools have no code that opens a file or a
  socket, so there is no option to get wrong. Asserted in
  [`examples/errors.sh`](../examples/errors.sh).
- **Entity expansion is bounded per document**, not per reference, so
  neither the exponential nor the quadratic blowup gets through.
- **Recursion is bounded**, so a deeply nested document returns an
  error rather than overflowing the stack — which would abort the
  process, and no caller can catch that.

Full reasoning:
<https://github.com/sebastienrousseau/oxml/blob/main/doc/SECURITY-MODEL.md>

## The JSON

The JSON-RPC and MCP layers belong to `rmcp`, the official SDK, which
is fuzzed and conformance-tested upstream and is the same code every
other Rust MCP server runs. Tool arguments are deserialised into typed
structs: a missing or mistyped argument is refused with `-32602`
before any tool runs.

Malformed input does not end the session. Over HTTP it is refused with
`400`; over stdio the line is skipped. A server that exited on it
could be killed by anything able to write a byte to it.

## The listener

`--transport streamable-http` and `--transport sse` open a TCP
listener. It binds `127.0.0.1` unless `--host` says otherwise, and it
does not authenticate: a routable deployment belongs behind a gateway
that does. The SDK refuses a `Host` header it does not expect, which
stops a page in a browser from reaching a local server through DNS
rebinding; binding every interface turns that check off, because
there is then no name to check against.

## What the server cannot do

- **Open a file.** Tools take document contents, never paths. A server
  that took paths would be a way to read any file the process can.
- **Make a network request.** The tools contain no network code. The
  listener answers requests; nothing in the process opens a
  connection outward.
- **Remember anything.** No state between calls, so nothing leaks
  between sessions and there is no cache to poison. An HTTP session is
  a routing key, not a store.

## Memory safety

`#![forbid(unsafe_code)]` in this crate. The dependency tree is the
SDK's -- `tokio`, `axum`, `serde` -- audited by `cargo audit` and
`cargo deny` in CI.

## What it does not protect you from

- **What the document says.** The server reports what is in the file.
  If a value is hostile to whatever the model does next, that is
  downstream of here.
- **A very large document.** It is parsed in full, in memory. A client
  that hands over a 2 GB document will find out.
- **Your own client's permissions.** This server does what it is asked
  with what it is given. What it is given is the client's decision.
