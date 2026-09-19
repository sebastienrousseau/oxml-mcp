// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! A whole session through an in-memory pipe.
//!
//! `tests/server.rs` drives the real binary over stdio, which is the
//! honest end-to-end check but can only assert on lines of text. These
//! hold a session with the SDK's own client, so what is asserted is
//! what a client sees: the negotiated protocol, the tool catalogue,
//! and results with their structured half.

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, ContentBlock, ProtocolVersion};
use rmcp::service::{RoleClient, RunningService};
use serde_json::{Map, Value, json};

/// A client connected to a fresh server over a duplex pipe.
async fn session() -> RunningService<RoleClient, ()> {
    let (client_io, server_io) = tokio::io::duplex(1 << 16);
    // `serve` returns once the handshake is done, so the server must
    // already be waiting when the client starts talking.
    drop(tokio::spawn(async move {
        if let Ok(server) = oxml_mcp::XmlServer::new().serve(server_io).await {
            let _ = server.waiting().await;
        }
    }));
    ().serve(client_io)
        .await
        .expect("client completes the handshake")
}

fn arguments(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => panic!("arguments must be an object"),
    }
}

fn text_of(content: &[ContentBlock]) -> &str {
    content
        .first()
        .and_then(ContentBlock::as_text)
        .map(|t| t.text.as_str())
        .expect("text content")
}

#[tokio::test]
async fn the_handshake_negotiates_a_current_revision() {
    let client = session().await;
    let info = client.peer_info().expect("initialize result");
    let server_info = info.server_info.as_ref().expect("serverInfo");
    assert_eq!(server_info.name, "oxml-mcp");
    assert_eq!(server_info.version, env!("CARGO_PKG_VERSION"));
    assert!(info.capabilities.tools.is_some(), "tools capability");
    // The SDK client asks for its latest handshake revision; the server
    // must agree to it rather than fall back.
    assert_eq!(info.protocol_version, ProtocolVersion::LATEST);
    let _ = client.cancel().await.expect("clean close");
}

#[tokio::test]
async fn the_catalogue_is_complete_and_annotated() {
    let client = session().await;
    let tools = client.list_all_tools().await.expect("tools/list");
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["xml_check", "xml_inspect", "xml_query", "xml_validate"]
    );
    for tool in &tools {
        assert!(tool.description.is_some(), "{} undescribed", tool.name);
        assert!(
            tool.output_schema.is_some(),
            "{} no outputSchema",
            tool.name
        );
        let a = tool.annotations.as_ref().expect("annotations");
        assert_eq!(a.read_only_hint, Some(true), "{} read-only", tool.name);
    }
    let _ = client.cancel().await.expect("clean close");
}

#[tokio::test]
async fn a_call_returns_text_and_structured_content() {
    let client = session().await;
    let result = client
        .call_tool(CallToolRequestParams::new("xml_query").with_arguments(
            arguments(json!({"xml": "<a><b>Dune</b></a>", "xpath": "//b"})),
        ))
        .await
        .expect("tools/call");
    assert_ne!(result.is_error, Some(true), "{result:?}");
    assert_eq!(text_of(&result.content), "Dune");
    assert_eq!(
        result.structured_content,
        Some(json!({"count": 1, "values": ["Dune"]}))
    );
    let _ = client.cancel().await.expect("clean close");
}

#[tokio::test]
async fn a_tool_failure_is_a_result_the_model_can_read() {
    let client = session().await;
    let result = client
        .call_tool(
            CallToolRequestParams::new("xml_check")
                .with_arguments(arguments(json!({"xml": "<a>"}))),
        )
        .await
        .expect("a bad document is a result, not a protocol error");
    assert_eq!(result.is_error, Some(true));
    assert!(text_of(&result.content).contains("not well-formed"));
    let _ = client.cancel().await.expect("clean close");
}

#[tokio::test]
async fn protocol_mistakes_are_json_rpc_errors() {
    let client = session().await;
    // A tool the server does not have is a result the model can read,
    // naming the tools it does have.
    let unknown = client
        .call_tool(CallToolRequestParams::new("no_such_tool"))
        .await
        .expect("a result, not a protocol error");
    assert_eq!(unknown.is_error, Some(true));
    let text = text_of(&unknown.content);
    assert!(text.contains("no_such_tool"), "{text}");
    assert!(text.contains("xml_query"), "{text}");

    // A required argument missing is a tool failure naming the field,
    // so the model can supply it.
    let missing = client
        .call_tool(
            CallToolRequestParams::new("xml_query")
                .with_arguments(arguments(json!({"xml": "<a/>"}))),
        )
        .await
        .expect("a result, not a protocol error");
    assert_eq!(missing.is_error, Some(true));
    assert!(text_of(&missing.content).contains("xpath"), "{missing:?}");
    let _ = client.cancel().await.expect("clean close");
}

#[tokio::test]
async fn every_advertised_tool_is_callable() {
    let client = session().await;
    let doc = "<r><t>x</t></r>";
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:element name="r"/></xs:schema>"#;
    for (name, args) in [
        ("xml_query", json!({"xml": doc, "xpath": "//t"})),
        ("xml_validate", json!({"xml": doc, "xsd": xsd})),
        ("xml_check", json!({"xml": doc})),
        ("xml_inspect", json!({"xml": doc})),
    ] {
        let result = client
            .call_tool(
                CallToolRequestParams::new(name)
                    .with_arguments(arguments(args)),
            )
            .await
            .unwrap_or_else(|e| panic!("{name} rejected: {e}"));
        assert_ne!(result.is_error, Some(true), "{name}: {result:?}");
        assert!(result.structured_content.is_some(), "{name} unstructured");
    }
    let _ = client.cancel().await.expect("clean close");
}
