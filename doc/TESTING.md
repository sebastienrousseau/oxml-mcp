<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Testing

## Unit and integration tests

```bash
cargo test
```

57 tests, covering the JSON parser and serialiser, request dispatch,
each tool, the transport loop, and the error paths.

The JSON layer carries most of them, because it is hand-written and its
input is untrusted. One bug it produced is worth recording: escaped
surrogate pairs were rejected, so a document containing an emoji failed
from any Python client — `json.dumps` escapes non-ASCII by default. The
fix was verified by reverting it and watching the test fail.

## The transport, without a subprocess

`serve` is generic over its two ends, so a test can hand it an
in-memory pipe:

```rust
let mut out = Vec::new();
oxml_mcp::serve(std::io::Cursor::new(input), &mut out);
```

`tests/serve.rs` uses that for the cases a real pipe will not produce
on demand — a writer that starts failing mid-session, input that stops
in the middle of a line, a malformed line followed by a good one. The
broken-pipe test counts *refused writes* rather than bytes written,
because a sink that rejects everything looks identical from the
outside whether the loop stopped or carried on; the refusals are what
differ. It was checked by making `serve` ignore the write error and
watching the count go from 1 to 2.

## The examples are end-to-end tests

```bash
./examples/run-all.sh
```

They drive the **real binary** over stdio and assert its replies, so
every request in the README fails CI when it stops being true.

| Script | Covers |
|---|---|
| `session.sh` | `initialize`, `tools/list`, all four tools, `count()`, an escaped surrogate pair |
| `errors.sh` | Malformed document, invalid expression, unknown tool, all four JSON-RPC codes, an external entity |

Writing them corrected a claim: an **unknown tool** is a JSON-RPC error
(`-32602`), not `isError: true`. MCP puts unknown tools and invalid
arguments in the protocol-error category and reserves `isError` for a
tool that ran and could not do the job. The assertion was written the
other way round and the server was right.

## What is not tested here

The parser, XPath and the conformance suite belong to `oxml` and are
tested there — 2,557 of 2,557 decided W3C conformance tests, zero
panics, fuzzing, Miri and property tests. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/TESTING.md>.
