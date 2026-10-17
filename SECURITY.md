<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Security Policy

## Supported versions

oxml-mcp is pre-1.0. Only the most recent `0.0.x` release receives security
fixes.

## Reporting a vulnerability

Please report privately, not as a public issue:

- GitHub's [private vulnerability reporting](https://github.com/sebastienrousseau/oxml-mcp/security/advisories/new)
- or email <sebastian.rousseau@gmail.com>

You should get an acknowledgement within 72 hours.

## Threat model

Tool XML parsing uses `Limits::strict()`, schema input is capped at
8 MiB, and XPath compilation/evaluation use explicit per-call limits.
Internal entities may expand within core budgets. No tool automatically
fetches XML documents, schemas or external entities over the network.

Stdio lines are capped at 16 MiB before the SDK can grow its line buffer
beyond that limit. Streamable HTTP uses the SDK's streaming 4 MiB request
body limit; legacy SSE uses Axum's default 2 MiB body extractor limit.
Stdio inherits the launching process's trust boundary. HTTP and SSE have
no application authentication and bind to loopback by default. A deployment
exposing them beyond loopback must supply authentication, TLS, request-size,
concurrency and session limits. Per-message limits do not bound the
number of simultaneous requests or total process memory.

**Memory safety.** This crate forbids unsafe Rust in its own source.
Dependencies and runtimes have separate safety obligations. This
does not rule out resource exhaustion, logic defects or protocol errors.

## What is not a vulnerability

- Rejecting a document other parsers accept, when the document is not
  well-formed. Please still report it — it is a bug, just not a
  security one.
