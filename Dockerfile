# syntax=docker/dockerfile:1.7
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# The MCP server image: one binary speaking MCP over stdio, for agents
# that do not have a Rust toolchain. Published to
# ghcr.io/sebastienrousseau/oxml-mcp:<version> by publish-mcp.yml on
# every release tag; `cargo install oxml-mcp` remains the native route.
#
# The entrypoint is stdio, which is what `docker run -i` gives an
# agent. To serve HTTP from the container instead, bind every
# interface so the port can be published:
#
#   docker run --rm -p 8000:8000 ghcr.io/sebastienrousseau/oxml-mcp:<version> \
#     --transport streamable-http --host 0.0.0.0 --port 8000
#
# Nothing authenticates that listener; publish it to a network you
# trust, or put a gateway in front.
#
# Both stages are pinned by digest so the same tag rebuilds to the same
# image, and the runtime stage carries no shell or package manager.

FROM rust:1.98-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS build

WORKDIR /src
COPY . .

ARG SOURCE_DATE_EPOCH
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-0}
ENV RUSTFLAGS="--remap-path-prefix=/src=/build --remap-path-prefix=/usr/local/cargo=/cargo"

RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:ce0d66bc0f64aae46e6a03add867b07f42cc7b8799c949c2e898057b7f75a151

LABEL org.opencontainers.image.title="oxml-mcp"
LABEL org.opencontainers.image.description="MCP server for XML: XPath queries, structure inspection, well-formedness checks and XSD validation, over stdio, streamable HTTP or SSE."
LABEL org.opencontainers.image.source="https://github.com/sebastienrousseau/oxml-mcp"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

COPY --from=build /src/target/release/oxml-mcp /usr/local/bin/oxml-mcp

USER nonroot:nonroot
# Only used with `--transport streamable-http` or `--transport sse`.
EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/oxml-mcp"]
