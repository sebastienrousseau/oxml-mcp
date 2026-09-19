// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! The `oxml-mcp` executable.
//!
//! Everything the server does lives in the library. This binary picks
//! the transport from the command line -- stdio by default, streamable
//! HTTP or the older HTTP+SSE on request -- and hands the library's
//! handler to it. `transport.rs` is the same file in every Rust server
//! of the suite.

#![forbid(unsafe_code)]

mod transport;

use std::process::ExitCode;

fn main() -> ExitCode {
    transport::run(
        "oxml-mcp",
        env!("CARGO_PKG_VERSION"),
        std::env::args().skip(1),
        oxml_mcp::XmlServer::new,
    )
}
