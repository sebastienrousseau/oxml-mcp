<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Testing

## Unit and integration tests

```bash
cargo test
```

48 tests, covering the JSON parser and serialiser, request dispatch,
each tool, and the error paths.

The JSON layer carries most of them, because it is hand-written and its
input is untrusted. One bug it produced is worth recording: escaped
surrogate pairs were rejected, so a document containing an emoji failed
from any Python client — `json.dumps` escapes non-ASCII by default. The
fix was verified by reverting it and watching the test fail.

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
tested there — 2,394 of 2,557 decided W3C conformance tests, zero
panics, fuzzing, Miri and property tests. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/TESTING.md>.
