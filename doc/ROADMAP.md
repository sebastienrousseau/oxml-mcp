<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Roadmap

## Where this is

A Model Context Protocol server exposing four read-only tools over
`oxml` and `xmlschema`: `xml_query`, `xml_validate`, `xml_check` and
`xml_inspect`. It speaks JSON-RPC over stdio, one message per line.

The point is that a model can ask questions of a document far larger
than its context without the document entering that context. A query
returns the matched values; an inspection returns the shape.

JSON parsing moved to `oxml-json` at 0.0.8, shared with `oxml-lsp`, so
the two servers cannot disagree about what a number means.

Documents are parsed in full before any tool answers.

## The order

**1. Streaming for `xml_check` and `xml_inspect`.** Both answer
questions a streaming parse can answer without holding the document,
and `oxml`'s `stream` module already reads from any `BufRead`. "The
document is larger than memory" is currently a reason not to use this
server; for those two tools it need not be.

`xml_query` and `xml_validate` genuinely need the tree.

**2. Position information for schema violations.** `xml_check`
reports line and column; `xml_query` reports the position in the
expression with a caret under it, added at 0.0.9. `xml_validate`
reports the path to the offending element -- `/library/book[2]/title`
-- which locates it in the tree but not in the source text a caller
would edit. Turning a path into a line and column needs the parser to
retain node positions, which is work in `oxml` rather than here.

**3. A document handle.** Every call re-parses the XML it is given. A
model that asks five questions of one document pays the parse five
times. A handle returned by a first call and accepted by later ones
would remove that, at the cost of server-side state -- which is why it
is third rather than first.

## What is deliberately absent

**Writing XML.** These tools read. A model that can mutate a document
through this server is a different threat model, and the read-only
surface is most of why this is safe to hand to one.

**Fetching documents.** The server never retrieves anything over the
network. XML that fetches what it references is how XXE works; a
server that helpfully resolved a URI would reintroduce the class of
bug the parser underneath was built to avoid.

**XSLT and XPath 2.0.** Neither exists in the library underneath.

## Non-goals

Becoming a general XML manipulation service. The value here is a small
read-only surface a model can be trusted with, and every addition
should be measured against that.
