// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml-mcp. All rights reserved.

//! Driving the MCP server without a client process.
//!
//! Run with:
//!
//! ```text
//! cargo run --example serve_a_session
//! ```
//!
//! The four operations are plain functions, so a program can call
//! them directly. The server that wraps them speaks MCP over anything
//! that reads and writes bytes, so a session can be held through an
//! in-memory pipe -- which is what makes it testable, and what this
//! example shows.

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Map, Value, json};

fn arguments(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("arguments are an object"),
    }
}

#[tokio::main]
async fn main() {
    let doc = "<library><book><title>Dune</title></book></library>";

    // The functions, without any protocol around them.
    let found = oxml_mcp::query(doc, "//title", &[]).expect("a valid query");
    println!("{found}");
    assert_eq!(found.values, ["Dune"]);

    let shape = oxml_mcp::inspect(doc).expect("a well-formed document");
    println!("{shape}");
    assert_eq!(shape.root, "library");

    let checked = oxml_mcp::check(doc).expect("a well-formed document");
    println!("{checked}");
    assert!(checked.well_formed);

    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="library"/></xs:schema>"#;
    let verdict = oxml_mcp::validate(doc, xsd).expect("a readable schema");
    println!("{verdict}");
    assert!(verdict.valid);

    // A failure is text a model can act on, not a panic.
    let fault = oxml_mcp::check("<a>").expect_err("not well-formed");
    println!("{fault}");
    assert!(fault.contains("line 1"));

    // The same four, as MCP tools over an in-memory pipe. A host
    // process would do this over the server's stdin and stdout.
    let (client_io, server_io) = tokio::io::duplex(1 << 16);
    // `serve` returns once the handshake is done, so the server side
    // runs in its own task and is already waiting when the client
    // starts talking.
    drop(tokio::spawn(async move {
        if let Ok(server) = oxml_mcp::XmlServer::new().serve(server_io).await {
            let _ = server.waiting().await;
        }
    }));
    let client = ().serve(client_io).await.expect("the client completes it");

    let info = client.peer_info().expect("initialize result");
    let server_info = info.server_info.as_ref().expect("serverInfo");
    println!(
        "connected to {} {} speaking MCP {}",
        server_info.name, server_info.version, info.protocol_version
    );

    let tools = client.list_all_tools().await.expect("tools/list");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    println!("tools: {}", names.join(", "));
    assert_eq!(names.len(), 4);

    let result = client
        .call_tool(CallToolRequestParams::new("xml_query").with_arguments(
            arguments(json!({"xml": doc, "xpath": "count(//book)"})),
        ))
        .await
        .expect("tools/call");
    println!(
        "count(//book) = {}",
        result.structured_content.expect("structured")["values"][0]
    );

    let _ = client.cancel().await.expect("clean close");
}
