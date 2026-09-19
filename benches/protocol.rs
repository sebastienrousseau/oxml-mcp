// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! What one tool call costs, by tool.
//!
//! An MCP client sends a request and waits, so the figure that matters
//! is latency per call, not throughput. The interesting quantity is
//! how much of that latency is *this* crate rather than `oxml`: each
//! tool is timed against the bare parse it contains, so the difference
//! is the tool's own work plus the text and structured result it
//! builds. The protocol framing belongs to the SDK and is not measured
//! here.
//!
//! Absolute figures describe the machine as much as the code -- see
//! `oxml`'s `doc/BENCHMARKS.md`. Compare runs, not numbers.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

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

/// A named call over a document.
type Case = (&'static str, String, Box<dyn Fn()>);

fn main() {
    let small = document(10);
    let large = document(2_000);
    let xsd = "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\
               <xs:element name=\"catalogue\"/></xs:schema>";

    let cases: Vec<Case> = vec![
        // The same tool over two document sizes says how the cost
        // splits between the tool and the parse.
        ("xml_check (10 entries)", small.clone(), {
            let d = small.clone();
            Box::new(move || {
                let _ = black_box(oxml_mcp::check(black_box(&d)));
            })
        }),
        ("xml_check (2,000 entries)", large.clone(), {
            let d = large.clone();
            Box::new(move || {
                let _ = black_box(oxml_mcp::check(black_box(&d)));
            })
        }),
        ("xml_inspect (2,000 entries)", large.clone(), {
            let d = large.clone();
            Box::new(move || {
                let _ = black_box(oxml_mcp::inspect(black_box(&d)));
            })
        }),
        ("xml_query (2,000 entries)", large.clone(), {
            let d = large.clone();
            Box::new(move || {
                let _ = black_box(oxml_mcp::query(
                    black_box(&d),
                    "//book/title",
                    &[],
                ));
            })
        }),
        ("xml_validate (2,000 entries)", large.clone(), {
            let d = large.clone();
            Box::new(move || {
                let _ = black_box(oxml_mcp::validate(black_box(&d), xsd));
            })
        }),
    ];

    println!("per call, fastest of 20 rounds\n");
    println!("{:<30} {:>10}  {:>10}", "tool", "time", "document");
    for (name, doc, call) in &cases {
        // A short call would be measured mostly by the clock, so
        // repeat the cheap ones enough to rise above its resolution.
        let reps = if doc.len() < 4_096 { 200 } else { 1 };
        let seconds = fastest(20, || {
            for _ in 0..reps {
                call();
            }
        }) / f64::from(reps);
        let micros = seconds * 1e6;
        println!("{name:<30} {micros:>8.1} us  {:>8} B", doc.len());
    }

    // How much of a tool call is the tool rather than the parse.
    //
    // Measured as a *paired* ratio, alternating the two inside one
    // loop. Timing them in separate loops and dividing the results
    // does not work here: consecutive runs of this benchmark disagree
    // by more than the effect, so whichever ran during a quieter
    // moment wins, and `xml_validate` came out faster than the parse
    // it contains. Alternating puts both under the same conditions.
    let (mut with_tool, mut bare) = (f64::INFINITY, f64::INFINITY);
    for _ in 0..40 {
        let a = Instant::now();
        let _ = black_box(oxml_mcp::check(black_box(&large)));
        with_tool = with_tool.min(a.elapsed().as_secs_f64());
        let b = Instant::now();
        let _ = black_box(oxml::parse(black_box(&large)));
        bare = bare.min(b.elapsed().as_secs_f64());
    }
    println!(
        "\ntool, paired against the parse it wraps:\n  \
         xml_check {:.0} us vs oxml::parse {:.0} us -- {:+.1}% for the tool",
        with_tool * 1e6,
        bare * 1e6,
        (with_tool / bare - 1.0) * 100.0
    );
}
