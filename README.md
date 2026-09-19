<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<h1 align="center">oxml-mcp</h1>

<p align="center">
  A Model Context Protocol server that lets a model <em>query</em> XML
  instead of reading it — powered by
  <a href="https://github.com/sebastienrousseau/oxml">oxml</a>, with zero
  <code>unsafe</code> code.
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/oxml-mcp/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/oxml-mcp/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/oxml-mcp"><img src="https://img.shields.io/crates/v/oxml-mcp.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/oxml-mcp"><img src="https://img.shields.io/badge/docs.rs-oxml--mcp-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/oxml-mcp"><img src="https://img.shields.io/ossf-scorecard/github.com/sebastienrousseau/oxml-mcp?style=for-the-badge&label=OpenSSF%20Scorecard&logo=openssf" alt="OpenSSF Scorecard" /></a>
  <a href="https://www.bestpractices.dev/projects/14312"><img src="https://img.shields.io/cii/level/14312?style=for-the-badge&label=OpenSSF%20Best%20Practices&logo=openssf" alt="OpenSSF Best Practices" /></a>
  <a href="https://glama.ai/mcp/servers/sebastienrousseau/oxml-mcp"><img src="https://glama.ai/mcp/servers/sebastienrousseau/oxml-mcp/badges/score.svg" alt="Glama MCP server score" /></a>
</p>

---

## Contents

**Getting started**

- [Why a model wants this](#why-a-model-wants-this) — the problem an XML tool solves for an LLM
- [Install](#install) — Cargo, from source
- [Transports](#transports) — stdio, streamable HTTP, SSE
- [Quick Start](#quick-start) — three lines, no client required
- [Configure](#configure) — Claude Desktop, and any MCP client

**The oxml ecosystem**

- [The oxml ecosystem](#the-oxml-ecosystem) — six crates, one version

**Reference**

- [Tools](#tools) — `xml_query`, `xml_validate`, `xml_check`, `xml_inspect`
- [Protocol](#protocol) — MCP `2025-11-25` and `2026-07-28`, and the two kinds of failure
- [Errors](#errors) — the two kinds, and which is which
- [Design](#design) — why four tools, and why documents are strings
- [Capabilities in 0.0.8](#capabilities-in-008) — release inventory
- [Ecosystem comparison](#ecosystem-comparison) — how this compares to the alternatives
- [Benchmarks](#benchmarks) — latency per request, measured in pairs

**Practical**

- [Examples](#examples) — a full session, and every error path
- [When not to use oxml-mcp](#when-not-to-use-oxml-mcp)
- [FAQ](#faq)
- [Development](#development)
- [Security](#security)
- [Documentation](#documentation)
- [Acknowledgements](#acknowledgements)
- [License](#license)

---

## Why a model wants this

A 40 MB XML file does not fit in a context window, and pasting a
fraction of it produces confident answers about the fraction.

`count(//record)` fits in twelve characters and returns a number. That
is the whole argument: give the model a query interface and the
document stays on disk.

The secondary argument is arithmetic. A model asked to count elements
in a document it can see will approximate. `xml_query` with
`count(//record)` will not.

## Install

```bash
cargo install oxml-mcp
```

Without a Rust toolchain, the same binary ships as an image on GHCR
(the [MCP registry](https://registry.modelcontextprotocol.io/) entry
points at it):

```bash
docker run --rm -i ghcr.io/sebastienrousseau/oxml-mcp:0.0.8
```

## Transports

One binary, three ways to reach it. Every server in the suite takes
the same flags.

| Command | Transport | Endpoint | Protocol revisions |
|---|---|---|---|
| `oxml-mcp` | stdio | stdin and stdout | `2024-11-05` to `2026-07-28` |
| `oxml-mcp --transport streamable-http --host 127.0.0.1 --port 8000` | Streamable HTTP | `http://127.0.0.1:8000/mcp` | `2025-11-25` and `2026-07-28` |
| `oxml-mcp --transport sse --port 8001` | HTTP+SSE (legacy) | `http://127.0.0.1:8001/sse`, `/messages/` | `2024-11-05` |

Streamable HTTP serves both current revisions on the one endpoint: a
client that sends `initialize` gets a session and an `Mcp-Session-Id`;
a client that names `2026-07-28` in each request's `_meta` is served
statelessly, with `server/discover` in place of the handshake.
Responses stream as server-sent events, and `GET /mcp` opens the
server-to-client stream. The SSE transport is the older one, for hosts
that still expect an `endpoint` event and a message URL.

The HTTP transports bind the loopback interface unless `--host` says
otherwise, and they do not authenticate. Put the server behind a
gateway you trust before binding a routable address. `--version` and
`--help` do what they say.

## Quick Start

The server speaks JSON-RPC 2.0 over stdio, so three lines are enough to
see it work — the handshake, then a call — and no client is required:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"me","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<library><book><title>Dune</title></book></library>","xpath":"//title"}}}' \
  | oxml-mcp | tail -n 1
```

```json
{"jsonrpc":"2.0","id":2,"result":{"resultType":"complete","content":[{"type":"text","text":"Dune"}],"structuredContent":{"count":1,"values":["Dune"]},"isError":false}}
```

For day-to-day use you want a client to do that for you — see
[Configure](#configure).

## Configure

Add it to your MCP client's server list. For Claude Desktop, in
`claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "oxml": {
      "command": "oxml-mcp"
    }
  }
}
```

For Claude Code:

```bash
claude mcp add oxml -- oxml-mcp
```

Without arguments the server speaks JSON-RPC 2.0 over stdin and
stdout, one message per line, which is what these clients expect. A
client that connects over HTTP instead points at a server started with
`--transport streamable-http` — see [Transports](#transports). There is
no configuration file.

## The oxml ecosystem

| Crate | What it is |
|---|---|
| [`oxml`](https://github.com/sebastienrousseau/oxml) | The library: parser, tree, XPath 1.0 |
| [`xmlschema`](https://github.com/sebastienrousseau/xmlschema) | XSD validation |
| [`oxml-cli`](https://github.com/sebastienrousseau/oxml-cli) | The command line |
| [`oxml-wasm`](https://github.com/sebastienrousseau/oxml-wasm) | WebAssembly bindings |
| **`oxml-mcp`** | **This crate — MCP server** |
| [`oxml-lsp`](https://github.com/sebastienrousseau/oxml-lsp) | Language Server Protocol server |

All six ship one version number, in steps of 0.0.1.

## Tools

### `xml_query`

Evaluate an XPath 1.0 expression and return the matching values.

| Argument | Type | |
|---|---|---|
| `xml` | string | The document |
| `xpath` | string | An XPath 1.0 expression |
| `namespaces` | object | Optional. Prefix-to-URI bindings for the expression |

```json
{"name":"xml_query","arguments":{"xml":"<r><t>Dune</t><t>Germinal</t></r>","xpath":"//t"}}
```

```
Dune
Germinal
```

One value per line. Expressions returning a number, string or boolean
return that value directly, so `count(//t)` gives `2`. Every tool also
returns its answer as `structuredContent`, against the `outputSchema`
it advertises — here `{"count": 2, "values": ["Dune", "Germinal"]}`.

### `xml_inspect`

Summarise a document's shape: root element, maximum depth, every
element name with its count, and the namespaces it uses.

```json
{"name":"xml_inspect","arguments":{"xml":"<r><t>x</t></r>"}}
```

```
Root element: r
Maximum depth: 3
Elements:
  r: 1
  t: 1
```

**Call this first.** A model that knows the element names can write a
query that works; one that guesses writes `//item` against a document
whose elements are called `record`.

### `xml_check`

Report whether a document is well-formed, with a line and column if it
is not.

```json
{"name":"xml_check","arguments":{"xml":"<a/>"}}
```

```
The document is well-formed (2 nodes).
```

### `xml_validate`

Validate against an XML Schema, returning every violation with the path
to the element it concerns.

| Argument | Type | |
|---|---|---|
| `xml` | string | The document |
| `xsd` | string | The schema |

## Protocol

MCP over JSON-RPC 2.0, implemented by [`rmcp`](https://crates.io/crates/rmcp),
the official Rust SDK. Two revisions are current and both are served:
`2025-11-25`, with an `initialize` handshake, and `2026-07-28`, which
has no handshake — each request names its revision in `_meta` and
`server/discover` describes the server. Older revisions back to
`2024-11-05` are accepted from a client that asks for them.

| Method | |
|---|---|
| `initialize` | Capabilities, server info, the negotiated revision |
| `server/discover` | The same, for the stateless revision |
| `tools/list` | The four tools: input schema, output schema, annotations |
| `tools/call` | Invoke one |
| `ping` | Answered |

Every tool is annotated read-only, idempotent and closed-world, so a
client can call it without asking.

The two kinds of failure are kept apart, because MCP distinguishes
them and a model only ever sees one of them.

**A tool that ran and could not do the job** — a malformed document, an
invalid expression — is a *successful* JSON-RPC response carrying
`isError: true`. The model sees the text and can correct itself.

**A request the protocol rejects** — malformed JSON, an unknown method,
a request with an `id` but no `method` — is a JSON-RPC error with the
standard code, or an HTTP status over HTTP. Nothing ran, and the
client handles it rather than the model.

| Situation | Reply |
|---|---|
| Malformed document | `result`, `isError: true` |
| Invalid XPath expression | `result`, `isError: true` |
| Schema violation | `result`, `isError: true`, with the violations as `structuredContent` |
| Missing or mistyped argument | `result`, `isError: true`, naming the field |
| Unknown tool | `result`, `isError: true`, naming the four that exist |
| Unknown method | `error`, `-32601` |
| `id` with no `method` | `error` |
| Malformed JSON | HTTP `415` over HTTP; skipped over stdio, the session continues |

## Errors

A malformed document is not a crash and not a protocol error:

```json
{"name":"xml_check","arguments":{"xml":"<a>"}}
```

```
{"content":[{"text":"…not well-formed…","type":"text"}],"isError":true}
```

The model reads that text and can fix the document or tell the user.

## Design

**Four tools, not fourteen.** Every tool description is in the model's
context on every request. A server with twenty narrow tools spends more
context describing itself than a document would.

**Documents are passed as strings, not paths.** The server never opens
a file. The client decides what the model may read, which is where that
decision belongs — a server that took paths would be a way to read any
file on the machine.

**One protocol implementation, not ours.** The JSON-RPC and MCP layers
are the official SDK's. Three protocol revisions and three transports
are a protocol project, and keeping a hand-written one honest against
them is not where the value of an XML server lies. What is this
crate's own is the four functions and the text a model reads.

## Capabilities in 0.0.8

- Four tools: query, inspect, check, validate
- XPath 1.0: ten axes, 25 functions, all four value types
- XSD validation
- JSON-RPC 2.0 over stdio, MCP `2024-11-05`
- Escaped surrogate pairs in JSON input, so a document containing an
  emoji works from a Python client
- No filesystem access, no network access

**Not yet:** resources, prompts, streaming, documents by path or URI.

## Ecosystem comparison

The alternatives are ways of getting XML in front of a model rather
than competing servers:

| Approach | Document size | Precision | Reaches the network |
|---|---|---|---|
| **`oxml-mcp`** | bounded by the tool call, not the context window | an XPath expression with an exact answer | **never** |
| Paste into the context | must fit, and costs tokens every turn | the model pattern-matches by eye | no |
| A filesystem or shell server | unbounded | whatever `grep` gives you | depends on the server |
| A generic HTTP fetch server | unbounded | none | **yes**, by design |

The last row is the one worth pausing on. A server that fetches is a
server that can be pointed at your internal network by a document it
was asked to read. The tools here have no code that opens a socket;
the only listener is the one you start with `--transport`, and it
only ever answers.

## Benchmarks

```bash
cargo bench --bench protocol
```

Latency per request, since an MCP client sends one and waits. The
JSON-RPC layer adds roughly 10–25% over the bare parse on a 200 KB
payload and is a few microseconds on a small one. See
[`doc/BENCHMARKS.md`](doc/BENCHMARKS.md), which also explains why that
comparison has to be measured in pairs.

## Examples

[`examples/`](examples/) drives the real binary over stdio and asserts
the responses, so the invocations in this README fail CI when they stop
being true.

| Example | What it shows |
|---|---|
| [`session.sh`](examples/session.sh) | A full session: initialise, list, call each tool |
| [`errors.sh`](examples/errors.sh) | Malformed documents, bad expressions, protocol errors |

## When not to use oxml-mcp

- **The document fits in context.** Paste it; a tool round-trip is
  slower and adds nothing.
- **You need the model to *write* XML.** These tools read.
- **You need XSLT or XPath 2.0.** Neither is available.
- **The document is larger than memory.** It is parsed in full.
- **You want the server to fetch documents.** It never will; that is
  the point.

## FAQ

### Why does the model have to pass the whole document every time?

Because the server holds no state between calls. That keeps it correct
when several clients share one binary, and it means there is no cache
to invalidate or leak between sessions.

For a large document this is genuinely wasteful, and a future release
may add a handle-based flow. Until then, `xml_inspect` once and a
precise `xml_query` beats several exploratory ones.

### Can it read a file from disk?

No, and it will not be able to. The server takes document *contents*.
A server that took paths would let any model with access to it read any
file the server process can — the client is where that decision
belongs.

### Is it safe to point at untrusted XML?

Yes. External entities are never dereferenced, so a document
containing `<!ENTITY xxe SYSTEM "file:///etc/passwd">` cannot make the
server read that file. Entity expansion and nesting depth are bounded.

### Why are there only four tools?

Every tool's description occupies context on every request. Four broad
tools cost less than twenty narrow ones and cover the same ground,
because XPath is already a query language.

### Does it work with clients other than Claude?

It implements MCP with no client-specific behaviour, over stdio and
both HTTP transports, so any compliant client should work. It is
checked against the Python SDK's client over both HTTP transports and
scores 100/100 with an independent MCP auditor in both current
protocol eras.

### My document contains an emoji and the call failed.

That was a bug, fixed in 0.0.3. Python's `json.dumps` escapes non-ASCII
by default, so an emoji arrives as a surrogate pair — `😀` —
and the JSON parser rejected escaped surrogate pairs. Any Python client
sending an emoji hit it.

### How do I query a document with namespaces?

Pass them with the query:

```json
{"name":"xml_query","arguments":{
  "xml":"…","xpath":"//m:item","namespaces":{"m":"urn:example"}}}
```

A prefix resolves against these bindings, **not** against the document,
so the same expression works across documents that spell the prefix
differently — only the URI has to match. An unbound prefix is an error
that names the argument to pass and points at `xml_inspect`.

`xml_inspect` reports the namespaces a document uses, which is what
makes the argument usable: a model cannot bind a URI it cannot see.

```
Namespaces (pass these to xml_query as `namespaces`):
  urn:example: 12 element(s)
```

An **unprefixed** name test matches only nodes in no namespace, which
is what XPath 1.0 specifies. `namespace-uri()` still works and needs no
binding.

### What happens if the model sends invalid JSON?

A JSON-RPC parse error, `-32700`. The server does not exit; the next
line is read as normal.

### Why is an unknown tool `isError` rather than a JSON-RPC error?

Until 0.0.8 it was `-32602`. In the stateless HTTP revision the SDK
carries that code as an HTTP 400, which a client reports as a
transport fault and a model never reads. A model that misspelt a tool
name is better served by text naming the four tools that exist. See
[Protocol](#protocol).

## Development

```bash
./scripts/gate.sh
```

That runs everything CI runs, in the order that fails fastest: format,
clippy, tests, rustdoc, the `#![forbid(unsafe_code)]` check, the
examples, the 95% coverage floor and an MSRV build. It pins the
toolchain rather than trusting `rust-toolchain.toml`, because a
`RUSTUP_TOOLCHAIN` in the environment silently overrides that file and
a lint that exists in one release and not another then makes a green
local run and a red CI one.

The individual steps, if you want them one at a time:

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo bench --bench protocol
OXML_MCP="$PWD/target/release/oxml-mcp" ./examples/run-all.sh
```

CI runs the same set on Linux, macOS and Windows.

## Security

The tools never open a file or a socket. External entities are never
dereferenced. Entity expansion and recursion are bounded.
`#![forbid(unsafe_code)]`. The HTTP listeners exist only when asked
for on the command line, bind loopback by default, and do not
authenticate — see [Transports](#transports).

The threat model is that both the document and the JSON around it are
hostile — the document because a model was asked to look at something
from the internet, and the JSON because it is the program's entire
input. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/SECURITY-MODEL.md>.

## Documentation

- [API documentation](https://docs.rs/oxml-mcp)
- [BENCHMARKS.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/doc/BENCHMARKS.md)
- [PROTOCOL.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/doc/PROTOCOL.md)
- [Architecture decision records](https://github.com/sebastienrousseau/oxml-mcp/blob/main/doc/adr/index.md)
- [TOOL-DESIGN.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/doc/TOOL-DESIGN.md)
- [SECURITY-MODEL.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/doc/SECURITY-MODEL.md)
- [TESTING.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/doc/TESTING.md)
- [CHANGELOG.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/CHANGELOG.md)
- [CONTRIBUTING.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/CONTRIBUTING.md)
- [SECURITY.md](https://github.com/sebastienrousseau/oxml-mcp/blob/main/SECURITY.md)

## Acknowledgements

`oxml-mcp` exists because of work that came before it:

- **[Anthropic](https://modelcontextprotocol.io/)** — for the Model
  Context Protocol specification this server implements.
- **[lxml](https://lxml.de/)** and
  **[libxml2](https://gitlab.gnome.org/GNOME/libxml2)** — the reference
  for what an XML toolkit should offer, and decades of hard-won
  correctness.
- **[W3C](https://www.w3.org/TR/1999/REC-xpath-19991116/)** — for the
  XPath 1.0 specification that `xml_query` follows.

## License

Licensed under either of Apache-2.0 ([LICENSE-APACHE](LICENSE-APACHE))
or MIT ([LICENSE-MIT](LICENSE-MIT)), at your option.
