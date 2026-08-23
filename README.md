<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<h1 align="center">oxml-mcp</h1>

<p align="center">
  A Model Context Protocol server for XML — XPath queries and XSD validation, for language models.
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/oxml-mcp/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/oxml-mcp/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/oxml-mcp"><img src="https://img.shields.io/crates/v/oxml-mcp.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/oxml-mcp"><img src="https://img.shields.io/badge/docs.rs-oxml-mcp-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://lib.rs/crates/oxml-mcp"><img src="https://img.shields.io/badge/lib.rs-oxml-mcp-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/sebastienrousseau/oxml-mcp"><img src="https://img.shields.io/ossf-scorecard/github.com/sebastienrousseau/oxml-mcp?style=for-the-badge&label=OpenSSF%20Scorecard&logo=openssf" alt="OpenSSF Scorecard" /></a>
</p>

---

## Why a model wants this

An LLM asked to pull a value out of a large XML document otherwise has
to read the whole thing into its context and pattern-match by eye. An
XPath tool turns that into a question with an exact answer, and the
document never has to fit in the context window.

## Install

```bash
cargo install --git https://github.com/sebastienrousseau/oxml-mcp
```

## Configure

```json
{
  "mcpServers": {
    "oxml": {
      "command": "oxml-mcp"
    }
  }
}
```

## Tools

| Tool | What it does |
|---|---|
| `xml_query` | Evaluate an XPath 1.0 expression and return the matches |
| `xml_validate` | Validate against an XSD; returns every violation with its path |
| `xml_check` | Report whether a document is well-formed, with line and column |
| `xml_inspect` | Summarise structure — element counts, depth, names present |

`xml_inspect` exists because a model querying an unfamiliar document
needs to know its shape before it can write a sensible XPath.

## Protocol

JSON-RPC 2.0 over stdio, protocol version `2024-11-05`.

```console
$ echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | oxml-mcp
{"jsonrpc":"2.0","id":1,"result":{"tools":[...]}}
```

## Design

JSON is read and written by a small module in this crate rather than a
serialisation dependency. The messages are few and their shapes are
fixed, and for a server meant to be audited that is a worthwhile trade.

Notifications — requests without an `id` — get no reply, because
answering one is a protocol error rather than merely noise.

## The oxml suite

Every member ships the **same version number**, so there is never a
compatibility table to consult. Versions advance in `0.0.1` steps along
the `0.0.x` line; `0.1.0` follows `0.0.999`.

| Crate | What it is |
|---|---|
| [`oxml`](https://github.com/sebastienrousseau/oxml) | Core — parser, tree, XPath 1.0 |
| [`oxml-cli`](https://github.com/sebastienrousseau/oxml-cli) | Command-line querying and validation |
| [`oxml-lsp`](https://github.com/sebastienrousseau/oxml-lsp) | Diagnostics for editors |
| [`oxml-mcp`](https://github.com/sebastienrousseau/oxml-mcp) | Model Context Protocol server |
| [`oxml-wasm`](https://github.com/sebastienrousseau/oxml-wasm) | WebAssembly bindings |
| [`xmlschema`](https://github.com/sebastienrousseau/xmlschema) | XSD validation |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). By participating you agree to
the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
