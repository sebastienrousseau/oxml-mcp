// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml-mcp. All rights reserved.

//! Driving the MCP server without a client.
//!
//! Run with:
//!
//! ```text
//! cargo run --example serve_a_session
//! ```
//!
//! A host starts `oxml-mcp` and speaks JSON-RPC to it over the
//! process's own streams, one message per line. `serve` is generic
//! over its two ends, so the same code can be driven from a buffer --
//! which is what makes it testable, and what this example shows.

use std::io::Cursor;

fn main() {
    // `handle_line` dispatches a single message. A request carries an
    // id and draws a reply; a notification has none and draws silence.
    let reply = oxml_mcp::handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
        .expect("a request with an id must be answered");
    println!("{reply}");
    assert!(reply.contains("\"id\":1"), "the reply echoes the request id");

    let silence = oxml_mcp::handle_line(r#"{"jsonrpc":"2.0","method":"initialized"}"#);
    assert!(
        silence.is_none(),
        "a notification has no id, so it draws no reply"
    );

    // `serve` is the loop around that: read a line, dispatch, write at
    // most one line back.
    let session: String = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        r#"{"jsonrpc":"2.0","method":"initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    ]
    .join("\n");

    let mut replies = Vec::new();
    oxml_mcp::serve(Cursor::new(session.into_bytes()), &mut replies);

    let transcript = String::from_utf8(replies).expect("replies are UTF-8");
    print!("{transcript}");
    assert_eq!(
        transcript.lines().count(),
        2,
        "two requests and one notification produce two replies"
    );
}
