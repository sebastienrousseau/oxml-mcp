// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! The `oxml-mcp` executable.
//!
//! Everything the server does lives in the library, which is what the
//! unit tests and benchmarks drive. This binary only supplies the two
//! ends of the pipe: an MCP client speaks over stdio.

#![forbid(unsafe_code)]

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    oxml_mcp::serve(stdin.lock(), stdout.lock());
}
