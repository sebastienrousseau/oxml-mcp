---
title: oxml-mcp — Model Context Protocol for XML
description: Model Context Protocol server exposing oxml's XML parsing, XPath, and XSD validation for AI agents.
hide:
  - navigation
  - toc
---

<section class="dot-hero" markdown>

# oxml-mcp

<p class="tagline">Model Context Protocol (MCP) server exposing oxml's fast XML parsing, XPath querying, and schema validation to AI agents.</p>

<div class="buttons">
  <a class="primary" href="TOOL-DESIGN/">Tools Reference →</a>
  <a href="https://github.com/sebastienrousseau/oxml-mcp">GitHub</a>
  <a href="PROTOCOL/">Protocol</a>
  <a href="SECURITY-MODEL/">Security Model</a>
</div>

</section>

## What's inside

<div class="grid cards" markdown>

- :material-robot-outline:{ .lg .middle } **Agent-ready XML tools**

    ---

    Structured tools for AI assistants: `xml_read`, `xml_xpath`, `xml_inspect`, and `xml_validate` with bounded token output.

    [→ Tool Design](TOOL-DESIGN.md)

- :material-cube-outline:{ .lg .middle } **Multi-transport MCP**

    ---

    Standard stdio for local desktop agents (Claude Desktop, Cursor) and streamable HTTP for remote servers.

    [→ Protocol Details](PROTOCOL.md)

- :material-shield-lock-outline:{ .lg .middle } **Hardened security**

    ---

    Strict entity expansion ceilings, configurable timeout guards, memory limits, and denial-of-service resilience.

    [→ Security Model](SECURITY-MODEL.md)

- :material-speedometer:{ .lg .middle } **Fast performance**

    ---

    Sub-millisecond query evaluation leveraging oxml's zero-unsafe arena allocation and string interning.

    [→ Benchmarks](BENCHMARKS.md)

</div>

## Quick start

Run with Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "oxml": {
      "command": "oxml-mcp"
    }
  }
}
```

Or run via Docker / OCI container:

```bash
docker run -i --rm ghcr.io/sebastienrousseau/oxml-mcp:latest
```

## Where to next

- [**Tool Design**](TOOL-DESIGN.md) — Tool signatures, input schemas, and expected outputs.
- [**Protocol**](PROTOCOL.md) — Transports, message formats, and lifecycle events.
- [**Security Model**](SECURITY-MODEL.md) — Safe execution bounds for agentic workflows.
- [**Assurance Case**](ASSURANCE-CASE.md) — Verification proofs and trust arguments.
- [**Testing**](TESTING.md) — Harness execution and MCP conformance testing.

## Current release

- Release notes: [GitHub Releases](https://github.com/sebastienrousseau/oxml-mcp/releases)
- Crates.io: [crates.io/crates/oxml-mcp](https://crates.io/crates/oxml-mcp)
- Repository: [sebastienrousseau/oxml-mcp](https://github.com/sebastienrousseau/oxml-mcp)
