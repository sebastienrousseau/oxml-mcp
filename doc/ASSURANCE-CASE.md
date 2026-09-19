<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Assurance case

An assurance case is an argument, supported by evidence, that the
software is adequately secure for what it does. This one is
deliberately short: the strongest security claim this project makes is
about what it *cannot* do.

## What this software is

`oxml-mcp` is a Model Context Protocol server exposing XML parsing, XPath and XSD validation over stdio, streamable HTTP and the older HTTP+SSE transport.

## What it consumes

Its inputs are JSON-RPC requests, and the XML and schema documents carried inside them — all untrusted, and in practice authored by a language model. Over HTTP they arrive from anyone who can reach the listener, which does not authenticate. The threat model assumes every one of them is
hostile: a document written specifically to crash the parser, exhaust
memory, or reach something it should not.

## The claim

**A hostile input can cause this software to return an error. It
cannot cause it to corrupt memory, execute code, exhaust the machine,
or reach the network or the filesystem.**

## The argument

### Memory safety is structural, not tested for

The tools have no filesystem and no network access. The server never fetches a document by path or URI, which is why documents are passed as strings; a server that fetches is one that can be aimed at an internal network by the document it was asked to read. The only socket is the listener the operator asks for on the command line, and it only answers.

### Resource exhaustion is bounded, not merely unlikely

Depth, entity expansion and input size are bounded by explicit limits
with documented defaults. Recursion is bounded because a stack
overflow aborts the process rather than unwinding, and no caller can
catch it.

### Correctness is measured against an external standard

The project does not grade its own homework. Where an independent
conformance suite exists it is run, its denominator is published
alongside its rate, and the result is ratcheted so an unreviewed change
in either direction fails the build.

## The evidence

- `#![forbid(unsafe_code)]`, checked by a CI job.
- 50 tests over each tool and its structured result, the command line, and every transport: a session through an in-memory pipe with the SDK's client, the binary over stdio, and the binary over both HTTP transports. The JSON-RPC layer is `rmcp`'s and is tested there.
- Line coverage gated at a 95% floor.
- A malformed request never costs a client its session: a body that is not JSON is refused over HTTP and skipped over stdio, and the next request is answered.
- The server scores 100/100 with an independent MCP auditor in both current protocol eras, and lists its tools with the reference Python client over both HTTP transports.

## What this case does *not* claim

- It does not claim the absence of defects. It claims that a defect of
  a particular class — memory corruption — is ruled out by
  construction, and that other classes are bounded and tested for.
- It does not claim the defaults are the tightest possible. They are
  chosen to accept every real document encountered; a service parsing
  untrusted XML under load should tighten them.
- It does not claim independent review. This project has one
  maintainer, and no third party has audited it. That is recorded here
  rather than left to be inferred.

## Reporting a problem with this case

If you can construct an input that violates the claim above, that is a
vulnerability. See [SECURITY.md](../SECURITY.md).
