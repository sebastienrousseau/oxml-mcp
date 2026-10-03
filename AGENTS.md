<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau <sebastian.rousseau@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# AGENTS.md

Invariants for AI-assisted contributions to `oxml-mcp`. Read this before changing anything.

Everything here applies equally to humans and automated agents. It is addressed to agents because agents can make breaking changes across multiple files before anyone notices.

## 1. Core Invariants

1. **Strict SemVer sequencing policy**: Public releases stay on the `0.0.x` line and increment strictly by `0.0.1`. Never manually edit version numbers outside the active release branch `feat/v<next-version>`. `v0.1.0` is forbidden until `v0.0.999` exists.
2. **Single Active Release PR Invariant**: Across all repositories, there MUST be at most ONE active pull request targeting `main`, which MUST be the release iteration branch `feat/v<next-version>`.
3. **Dual licensing**: The repository is dual-licensed under Apache-2.0 OR MIT. All files must declare an SPDX license header.
4. **Single source of truth**: The version in `Cargo.toml` is the single source of truth. It must agree with `glama.json`, `server.json`, and `CHANGELOG.md` (verified by `scripts/check-mcp-manifests.sh`).
5. **Zero unsafe code**: The entire crate enforces `#![forbid(unsafe_code)]`. No unsafe blocks are permitted under any circumstances.
6. **Deterministic XML processing**: XPath evaluations, structure inspections, well-formedness checks, and XSD validations must remain pure, offline, and deterministic.

## 2. Before You Claim To Be Done (Verification Gates)

Before concluding any task or preparing a commit, run:

```console
make check
```

Or run the individual gates:

```console
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
bash scripts/check-mcp-manifests.sh
python3 scripts/check-figures.py
python3 scripts/check-package.py
```

All unit tests and example assertions must pass with 0 failures.

## 3. Hygiene First

Before any feature, fix, or release work, check repository health:
1. Verify CI is green on `main`.
2. Ensure linter and formatter pass without warnings or new suppressions.
3. Preserve all documentation figures, architecture records, and test counts.

## 4. Things That Look Like Bugs and Are Not

- **Stateless MCP revision 2026-07-28**: When called without a prior handshake, requests must supply `_meta` containing `io.modelcontextprotocol/protocolVersion`. This is intentional and compliant with the MCP 2026-07-28 specification.
- **Transports**: `oxml-mcp` serves stdio, streamable HTTP, and SSE without authentication. Loopback binding is enforced by default.
