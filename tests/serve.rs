// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! `serve` driven through an in-memory pipe.
//!
//! `tests/server.rs` covers the same loop through a real process,
//! which is the honest end-to-end check but can only supply a pipe
//! that behaves. These cover the ends it cannot: a writer that starts
//! failing mid-session, and input that stops in the middle of a line.
//! Both are things a real client does -- it disconnects -- and the
//! loop must end rather than spin or panic.

use std::io::{self, Cursor, Write};

/// A writer that fails once it has accepted `budget` bytes.
///
/// A closed pipe looks exactly like this from the server's side.
struct Failing {
    written: Vec<u8>,
    budget: usize,
    /// Writes attempted after the budget ran out.
    ///
    /// Counting them is what makes the test discriminate: a sink that
    /// refuses bytes looks the same from the outside whether the loop
    /// stopped or carried on regardless, because either way nothing
    /// more arrives. The refusals do differ.
    refused: usize,
}

impl Write for Failing {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written.len() >= self.budget {
            self.refused += 1;
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone"));
        }
        let take = buf.len().min(self.budget - self.written.len());
        self.written.extend_from_slice(&buf[..take]);
        Ok(take)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// One request line with the given id.
fn line(id: u32) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"initialize","params":null}}"#
    )
}

#[test]
fn every_request_draws_exactly_one_reply_line() {
    let input = format!("{}\n{}\n{}\n", line(1), line(2), line(3));
    let mut out = Vec::new();
    oxml_mcp::serve(Cursor::new(input.as_bytes()), &mut out);
    let text = String::from_utf8(out).expect("utf-8");
    assert_eq!(text.lines().count(), 3, "got {text}");
    for (i, reply) in text.lines().enumerate() {
        assert!(
            reply.contains(&format!("\"id\":{}", i + 1)),
            "reply {i} was {reply}"
        );
    }
}

#[test]
fn blank_lines_and_notifications_draw_no_reply() {
    let input = format!(
        "\n   \n{}\n{}\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        line(1)
    );
    let mut out = Vec::new();
    oxml_mcp::serve(Cursor::new(input.as_bytes()), &mut out);
    let text = String::from_utf8(out).expect("utf-8");
    assert_eq!(text.lines().count(), 1, "got {text}");
}

#[test]
fn a_broken_pipe_ends_the_session_rather_than_looping() {
    // Room for the first reply and nothing after it.
    let first = oxml_mcp::handle_line(&line(1)).expect("a reply");
    let input = format!("{}\n{}\n{}\n", line(1), line(2), line(3));
    let mut sink = Failing {
        written: Vec::new(),
        budget: first.len() + 1,
        refused: 0,
    };
    oxml_mcp::serve(Cursor::new(input.as_bytes()), &mut sink);
    assert_eq!(
        sink.refused, 1,
        "the loop should stop at the first refusal, not try every line"
    );
}

#[test]
fn input_ending_mid_line_ends_the_session() {
    // No trailing newline: `lines()` still yields the partial line, and
    // a well-formed request on it deserves its reply.
    let mut out = Vec::new();
    oxml_mcp::serve(Cursor::new(line(1).into_bytes()), &mut out);
    let text = String::from_utf8(out).expect("utf-8");
    assert_eq!(text.lines().count(), 1, "got {text}");
}

#[test]
fn malformed_input_does_not_end_the_session() {
    // One bad line must not cost the client the rest of its session.
    let input = format!("not json\n{}\n", line(2));
    let mut out = Vec::new();
    oxml_mcp::serve(Cursor::new(input.as_bytes()), &mut out);
    let text = String::from_utf8(out).expect("utf-8");
    assert_eq!(text.lines().count(), 2, "got {text}");
    assert!(text.lines().next().expect("a line").contains("error"));
}
