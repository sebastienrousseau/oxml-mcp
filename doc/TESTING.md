<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Testing

## Unit and integration tests

```bash
cargo test
```

50 tests, covering each tool and its structured result, the
command line, and every transport: a session held with the SDK's own
client through an in-memory pipe, the binary driven over stdio, and
the binary driven over both HTTP transports by a small hand-written
HTTP/1.1 client. The JSON-RPC layer belongs to `rmcp` and is tested
there.

## A session without a process

`tests/serve.rs` connects the SDK's client to the server through
`tokio::io::duplex`, so what is asserted is what a client sees: the
negotiated revision, the catalogue with its schemas and annotations,
a result with its structured half, and a tool failure arriving as an
`isError` result rather than a protocol error.

Its first version deadlocked. `serve` returns only once the handshake
is done, so a test that awaited the server before starting the client
waited for a message nobody would send. The server side now runs in
its own task, which is also how the example does it.

## The wire, as a client sees it

`tests/http.rs` starts the binary on `--port 0`, reads the port it
announces, and speaks HTTP/1.1 over a plain socket, decoding chunked
bodies and server-sent events by hand. A client library would have
hidden the details this file exists to check: the `Mcp-Session-Id`
on an `initialize` reply, the priming event that carries no data, the
`405` a stateless `GET` gets, the `-32020` a mirrored routing header
draws when it disagrees with the body, and the `endpoint` event that
opens the older SSE transport.

Two of its expectations were wrong when written and the server was
right: the `2026-07-28` revision requires `clientCapabilities` in
`_meta` beside the protocol version, and it requires the `Mcp-Method`
header on every request. Both are now documented because a test
found them, not the other way round.

## The examples are end-to-end tests

```bash
./examples/run-all.sh
```

They drive the **real binary** over stdio and assert its replies, so
every request in the README fails CI when it stops being true.

| Script | Covers |
|---|---|
| `session.sh` | The handshake in both current revisions, `server/discover`, `tools/list` with schemas and annotations, all four tools, `count()`, an escaped surrogate pair |
| `errors.sh` | Malformed document, invalid expression, schema violation, unknown tool, missing argument, unknown method, a line that is not JSON, an external entity |

Writing them once corrected a claim the other way: an **unknown
tool** was a JSON-RPC error (`-32602`), and the assertion said
`isError`. From 0.0.9 the assertion is right and the server changed:
the stateless HTTP revision carries `-32602` as an HTTP 400, which no
model reads, so an unknown tool is now a result naming the tools that
exist.

## Fuzzing

```bash
cargo +nightly fuzz run tools
```

`tools` feeds arbitrary documents, expressions and schemas to the four
functions, which is the surface still written here now that the
JSON-RPC layer is the SDK's. One input, split on NUL bytes, exercises
every tool.

The previous target, `handle_line`, ran 4,136,133 executions without a
crash against the hand-written JSON-RPC handler; that handler is gone
and so is the figure. CI runs the target for 300 seconds on every pull
request, seeded from the tracked files in `fuzz/seeds/` — the grown
corpus is build output and is not tracked, so a run starts from the
same place every time rather than from whatever a previous run
happened to discover. A crash input is uploaded as a build artefact,
because knowing only that something broke is not much use.

## Coverage

Line coverage is gated in CI at a 95% floor. **Branch coverage is
86.6%**, gated at 80.

Branch coverage needs a nightly toolchain: `cargo llvm-cov --branch`
does not build on the version this project pins. It was recorded as
unmeasurable for a while on the strength of that one failure, which
was a conclusion drawn from a single attempt.

## What is not tested here

The parser, XPath and the conformance suite belong to `oxml` and are
tested there (this crate fuzzes its own surface — see above) — 2,557 of 2,557 decided W3C conformance tests, zero
panics, fuzzing, Miri and property tests. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/TESTING.md>.
