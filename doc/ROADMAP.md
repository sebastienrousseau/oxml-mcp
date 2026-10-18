<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Roadmap

## Where this is

A Model Context Protocol server exposing five read-only tools over
`oxml` and `xmlschema`: `xml_query`, `xml_validate`, `xml_check`,
`xml_inspect` and `xml_format`. It speaks MCP over stdio, streamable HTTP and the
older HTTP+SSE transport.

The point is that a model can ask questions of a document far larger
than its context without the document entering that context. A query
returns the matched values; an inspection returns the shape.

JSON parsing moved to `oxml-json` at 0.0.8. MCP now uses the protocol
SDK's JSON stack; LSP continues to use `oxml-json`.

Documents are parsed in full under the strict core profile before any
tool answers. Each query has a fresh evaluation budget.

## The order

**1. Streaming for `xml_check` and `xml_inspect`.** Both answer
questions a streaming parse can answer without holding the document,
and `oxml`'s `stream` module already reads from any `BufRead`. "The
document is larger than memory" is currently a reason not to use this
server; for those two tools it need not be.

`xml_query` and `xml_validate` genuinely need the tree.

**Implemented: position information in tool errors.** XML parse errors
include their line and column. Invalid XPath expressions include the
expression's line and column; validation violations name their paths.

**3. A document handle.** Every call re-parses the XML it is given. A
model that asks five questions of one document pays the parse five
times. A handle returned by a first call and accepted by later ones
would remove that, at the cost of server-side state -- which is why it
is third rather than first.

## What is deliberately absent

**Structural XML editing.** `xml_format` serializes a parsed document,
but no tool changes element or attribute data or persists edits. Adding
mutation or filesystem writes would change the threat model.

**Fetching documents.** The server never retrieves anything over the
network. XML that fetches what it references is how XXE works; a
server that helpfully resolved a URI would reintroduce the class of
bug the parser underneath was built to avoid.

**XSLT and XPath 2.0.** Neither exists in the library underneath.

## Non-goals

Becoming a general XML manipulation service. The value here is a small
read-only surface a model can be trusted with, and every addition
should be measured against that.
