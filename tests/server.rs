// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! The server over a real pipe.
//!
//! The dispatch layer is unit-tested in the binary. What these cover is
//! the transport around it: line framing, flushing, which inputs draw a
//! reply at all, and that the process exits cleanly at end of input.
//! None of that is reachable from a unit test, and all of it is what an
//! MCP client depends on.

use std::io::Write as _;
use std::process::{Command, Stdio};

/// Send `lines` to the server and collect the replies.
fn converse(lines: &[&str]) -> Vec<String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxml-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("server starts");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for l in lines {
            writeln!(stdin, "{l}").expect("write");
        }
    }
    // Dropping stdin signals end of input; the server must then exit.
    let out = child.wait_with_output().expect("server exits");
    assert!(out.status.success(), "server exited with {}", out.status);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":");
    let rest = &line[line.find(&needle)? + needle.len()..];
    let rest = rest.trim_start().strip_prefix('"')?;
    rest.find('"').map(|end| &rest[..end])
}

#[test]
fn one_request_draws_exactly_one_line() {
    let replies =
        converse(&[r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#]);
    assert_eq!(replies.len(), 1, "{replies:?}");
    assert!(replies[0].contains("\"id\":1"), "{}", replies[0]);
    assert_eq!(field(&replies[0], "protocolVersion"), Some("2024-11-05"));
}

#[test]
fn a_session_is_answered_in_order() {
    let replies = converse(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<a><b>Dune</b></a>","xpath":"//b"}}}"#,
    ]);
    // Three requests, one notification: three replies.
    assert_eq!(replies.len(), 3, "{replies:?}");
    assert!(replies[0].contains("\"id\":1"));
    assert!(replies[1].contains("\"id\":2"));
    assert!(replies[2].contains("\"id\":3"));
    assert!(replies[2].contains("Dune"), "{}", replies[2]);
}

#[test]
fn blank_lines_are_skipped_rather_than_answered() {
    // Some clients pad the stream; a reply to a blank line would
    // desynchronise the whole conversation.
    let replies = converse(&[
        "",
        "   ",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        "",
    ]);
    assert_eq!(replies.len(), 1, "{replies:?}");
}

#[test]
fn a_notification_alone_draws_no_reply_and_exits_cleanly() {
    let replies = converse(&[r#"{"jsonrpc":"2.0","method":"initialized"}"#]);
    assert!(replies.is_empty(), "{replies:?}");
}

#[test]
fn malformed_input_does_not_stop_the_server() {
    // The connection must survive a bad line: an MCP client would
    // otherwise see the whole server die on one typo.
    let replies = converse(&[
        "this is not json",
        r#"{"jsonrpc":"2.0","id":2,"method":"initialize"}"#,
    ]);
    assert_eq!(replies.len(), 2, "{replies:?}");
    assert!(replies[0].contains("-32700"), "{}", replies[0]);
    assert!(replies[1].contains("\"id\":2"), "{}", replies[1]);
}

#[test]
fn no_input_at_all_exits_successfully() {
    assert!(converse(&[]).is_empty());
}

#[test]
fn a_document_containing_newlines_stays_on_one_line() {
    // The transport is line-delimited; an embedded newline in the
    // response would split it into two unparseable halves.
    let replies = converse(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"xml_inspect","arguments":{"xml":"<a>\n<b/>\n<b/>\n</a>"}}}"#,
    ]);
    assert_eq!(replies.len(), 1, "{replies:?}");
    assert!(replies[0].starts_with('{') && replies[0].ends_with('}'));
}
