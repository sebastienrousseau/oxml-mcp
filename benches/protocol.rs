// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! What one MCP request costs, by method.
//!
//! An MCP client sends a request and waits, so the figure that matters
//! is latency per call, not throughput. The interesting quantity is
//! how much of that latency is *this* crate rather than `oxml`:
//! `initialize` and `tools/list` do no XML work at all, so they price
//! the JSON-RPC layer on its own, and the `xml_*` calls price it plus
//! the parse. If a tool call costs barely more than `tools/list`, the
//! protocol layer is where the time goes.
//!
//! `serve` is measured separately over an in-memory pipe, which is the
//! only way to see the line framing and flushing without a subprocess
//! in the way.
//!
//! Absolute figures describe the machine as much as the code -- see
//! `oxml`'s `doc/BENCHMARKS.md`. Compare runs, not numbers.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

/// A JSON string literal, escaped.
///
/// The benchmark writes the wire format by hand rather than through
/// the crate's own JSON module: driving the server with bytes a client
/// would actually send keeps the request-parsing cost in the
/// measurement, where it belongs.
fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One JSON-RPC request line.
fn request(method: &str, params: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{params}}}"#
    )
}

/// A `tools/call` line for `tool` with the given arguments.
fn tool_call(tool: &str, args: &[(&str, &str)]) -> String {
    let mut body = String::new();
    for (i, (k, v)) in args.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        let _ = write!(body, "{}:{}", quoted(k), quoted(v));
    }
    request(
        "tools/call",
        &format!(r#"{{"name":{},"arguments":{{{body}}}}}"#, quoted(tool)),
    )
}

/// A document of `n` entries.
fn document(n: usize) -> String {
    let mut s = String::from("<?xml version=\"1.0\"?>\n<catalogue>\n");
    for i in 0..n {
        let _ = write!(
            s,
            "  <book id=\"b{i}\" lang=\"en\">\n    \
             <title>Title {i}</title>\n    \
             <pages>{i}</pages>\n  </book>\n"
        );
    }
    s.push_str("</catalogue>\n");
    s
}

/// The fastest of `rounds` runs.
///
/// Contention can only make a run slower, so the fastest is the least
/// perturbed sample. A mean would mostly measure whatever else the
/// machine was doing.
fn fastest(rounds: usize, mut f: impl FnMut()) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..rounds {
        let start = Instant::now();
        f();
        best = best.min(start.elapsed().as_secs_f64());
    }
    best
}

fn main() {
    let small = document(10);
    let large = document(2_000);
    let xsd = "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\
               <xs:element name=\"catalogue\"/></xs:schema>";

    let cases: Vec<(String, String)> = vec![
        // No XML at all: the JSON-RPC layer on its own.
        ("initialize".to_owned(), request("initialize", "null")),
        ("tools/list".to_owned(), request("tools/list", "null")),
        ("malformed line".to_owned(), "not json at all".to_owned()),
        // The same tool over two document sizes says how the cost
        // splits between the protocol and the parse.
        (
            "xml_check (10 entries)".to_owned(),
            tool_call("xml_check", &[("xml", &small)]),
        ),
        (
            "xml_check (2,000 entries)".to_owned(),
            tool_call("xml_check", &[("xml", &large)]),
        ),
        (
            "xml_inspect (2,000 entries)".to_owned(),
            tool_call("xml_inspect", &[("xml", &large)]),
        ),
        (
            "xml_query (2,000 entries)".to_owned(),
            tool_call(
                "xml_query",
                &[("xml", &large), ("xpath", "//book/title")],
            ),
        ),
        (
            "xml_validate (2,000 entries)".to_owned(),
            tool_call("xml_validate", &[("xml", &large), ("xsd", xsd)]),
        ),
    ];

    println!("per request, fastest of 20 rounds\n");
    println!("{:<30} {:>10}  {:>10}", "request", "time", "line");
    for (name, line) in &cases {
        // A short request would be measured mostly by the clock, so
        // repeat the cheap ones enough to rise above its resolution.
        let reps = if line.len() < 4_096 { 200 } else { 1 };
        let seconds = fastest(20, || {
            for _ in 0..reps {
                let _ = black_box(oxml_mcp::handle_line(black_box(line)));
            }
        }) / f64::from(reps);
        let micros = seconds * 1e6;
        println!("{name:<30} {micros:>8.1} us  {:>8} B", line.len());
    }

    // How much of a tool call is the protocol rather than the parse.
    //
    // Measured as a *paired* ratio, alternating the two inside one
    // loop. Timing them in separate loops and dividing the results
    // does not work here: consecutive runs of this benchmark disagree
    // by more than the effect, so whichever ran during a quieter
    // moment wins, and `xml_validate` came out faster than the parse
    // it contains. Alternating puts both under the same conditions.
    let call = tool_call("xml_check", &[("xml", &large)]);
    let (mut with_protocol, mut bare) = (f64::INFINITY, f64::INFINITY);
    for _ in 0..40 {
        let a = Instant::now();
        let _ = black_box(oxml_mcp::handle_line(black_box(&call)));
        with_protocol = with_protocol.min(a.elapsed().as_secs_f64());
        let b = Instant::now();
        let _ = black_box(oxml::parse(black_box(&large)));
        bare = bare.min(b.elapsed().as_secs_f64());
    }
    println!(
        "\nprotocol layer, paired against the parse it wraps:\n  \
         xml_check {:.0} us vs oxml::parse {:.0} us -- {:+.1}% for \
         JSON-RPC",
        with_protocol * 1e6,
        bare * 1e6,
        (with_protocol / bare - 1.0) * 100.0
    );

    // The transport, which only a library target can reach: `serve`
    // reading many lines from one buffer and writing to another.
    let session: String = cases
        .iter()
        .map(|(_, line)| format!("{line}\n"))
        .collect::<Vec<_>>()
        .concat();
    let bytes = session.len();
    let seconds = fastest(20, || {
        let mut sink = Vec::with_capacity(1 << 16);
        oxml_mcp::serve(
            std::io::Cursor::new(black_box(session.as_bytes())),
            &mut sink,
        );
        let _ = black_box(sink);
    });
    println!(
        "\nserve, {} requests over an in-memory pipe: {:.1} us ({bytes} B in)",
        cases.len(),
        seconds * 1e6
    );
}
