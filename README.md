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
</p>

---

## Contents

- [Why a model wants this](#why-a-model-wants-this)
- [Install](#install)
- [Configure](#configure)
- [The oxml ecosystem](#the-oxml-ecosystem)
- [Tools](#tools)
- [Protocol](#protocol)
- [Errors](#errors)
- [Design](#design)
- [Capabilities in 0.0.6](#capabilities-in-006)
- [Examples](#examples)
- [When not to use oxml-mcp](#when-not-to-use-oxml-mcp)
- [FAQ](#faq)
- [Development](#development)
- [Security](#security)
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

The server speaks JSON-RPC 2.0 over stdin and stdout, one message per
line. It takes no arguments and reads no configuration file.

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
return that value directly, so `count(//t)` gives `2`.

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

JSON-RPC 2.0 over stdio, one message per line, MCP protocol version
`2024-11-05`.

| Method | |
|---|---|
| `initialize` | Returns capabilities and server info |
| `tools/list` | The four tools with their JSON schemas |
| `tools/call` | Invoke one |

The two kinds of failure are kept apart, because MCP distinguishes
them and a model only ever sees one of them.

**A tool that ran and could not do the job** — a malformed document, an
invalid expression — is a *successful* JSON-RPC response carrying
`isError: true`. The model sees the text and can correct itself.

**A request the protocol rejects** — malformed JSON, an unknown method,
an unknown tool, a request with an `id` but no `method` — is a JSON-RPC
error with the standard code. The tool never ran, and the client
handles it rather than the model.

| Situation | Reply |
|---|---|
| Malformed document | `result`, `isError: true` |
| Invalid XPath expression | `result`, `isError: true` |
| Schema violation | `result`, `isError: true` |
| Unknown tool | `error`, `-32602` |
| Unknown method | `error`, `-32601` |
| `id` with no `method` | `error`, `-32600` |
| Malformed JSON | `error`, `-32700` |

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

**No dependencies.** JSON parsing and serialisation are in
`src/json.rs`, about 300 lines. For a program whose entire input is
untrusted JSON arriving on stdin, a dependency tree is a liability, and
this one has none beyond `oxml` and `xmlschema`.

## Capabilities in 0.0.6

- Four tools: query, inspect, check, validate
- XPath 1.0: ten axes, 25 functions, all four value types
- XSD validation
- JSON-RPC 2.0 over stdio, MCP `2024-11-05`
- Escaped surrogate pairs in JSON input, so a document containing an
  emoji works from a Python client
- No filesystem access, no network access

**Not yet:** resources, prompts, streaming, documents by path or URI.

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

It implements MCP over stdio with no client-specific behaviour, so any
compliant client should work.

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

### Why is an unknown tool an error rather than `isError`?

Because the tool never ran. MCP puts unknown tools and invalid
arguments in the JSON-RPC error category and reserves `isError` for a
tool that executed and could not do the job — the first is the
client's problem, the second is the model's. See
[Protocol](#protocol).

## Development

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
./examples/run-all.sh
```

## Security

No filesystem access. No network access. External entities never
dereferenced. Entity expansion and recursion bounded.
`#![forbid(unsafe_code)]`, and no dependencies beyond `oxml` and
`xmlschema`.

The threat model is that both the document and the JSON around it are
hostile — the document because a model was asked to look at something
from the internet, and the JSON because it is the program's entire
input. See
<https://github.com/sebastienrousseau/oxml/blob/main/doc/SECURITY-MODEL.md>.

## License

Licensed under either of Apache-2.0 ([LICENSE-APACHE](LICENSE-APACHE))
or MIT ([LICENSE-MIT](LICENSE-MIT)), at your option.
