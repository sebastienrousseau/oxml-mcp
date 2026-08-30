// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! `oxml-mcp` — a Model Context Protocol server for XML.
//!
//! Speaks JSON-RPC 2.0 over stdio, which is what MCP clients expect.
//! Four tools: parse, query, validate, and inspect.
//!
//! Why a model wants this: an LLM asked to pull a value out of a large
//! XML document otherwise has to read the whole thing into its
//! context and pattern-match by eye. An `XPath` tool turns that into a
//! question with an exact answer, and the document never needs to fit
//! in the context window.

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::io::{BufRead, Write};

use oxml_json::{self as json, Json};

/// The MCP revision this server implements.
///
/// A client that speaks a different one is told this in the
/// `initialize` reply and decides for itself whether to continue.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Serve the protocol over a byte stream until input ends.
///
/// This is the whole server: read a line, dispatch it, write at most
/// one line back. It is generic over its two ends so that a test or a
/// benchmark can drive it through an in-memory pipe, and `main` can
/// hand it stdin and stdout.
///
/// A write failure ends the loop rather than being reported: the peer
/// has gone, and there is nowhere left to report it to. Input that is
/// not valid JSON is answered with a JSON-RPC error, not a
/// disconnection -- one malformed line must not take down a session.
pub fn serve<R: BufRead, W: Write>(input: R, mut output: W) {
    for line in input.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Some(response) = handle_line(&line) else {
            // A notification: no id, so no reply. Responding anyway
            // is a protocol error, not merely noise.
            continue;
        };
        if writeln!(output, "{response}").is_err() {
            break;
        }
        let _ = output.flush();
    }
}

/// Dispatch one JSON-RPC line, returning the reply if there is one.
///
/// `None` means the line was a notification, which by JSON-RPC 2.0
/// draws no response at all. Every other outcome -- including a
/// malformed request or a tool that failed -- produces a reply, so a
/// client is never left waiting.
#[must_use]
pub fn handle_line(line: &str) -> Option<String> {
    let request = match json::parse(line) {
        Ok(r) => r,
        Err(e) => {
            return Some(error_response(
                &Json::Null,
                -32700,
                &format!("parse error: {e}"),
            ));
        }
    };

    // No id means a notification: no reply, even for a bad one.
    let id = request.get("id").cloned()?;

    let Some(method) = request.get("method").and_then(Json::as_str) else {
        // An id was supplied, so silence would leave the client
        // waiting for a response that is never coming.
        return Some(error_response(&id, -32600, "missing method"));
    };

    Some(match method {
        "initialize" => initialize(&id),
        "tools/list" => tools_list(&id),
        "tools/call" => tools_call(&id, &request),
        other => {
            error_response(&id, -32601, &format!("unknown method `{other}`"))
        }
    })
}

fn ok_response(id: &Json, result: Json) -> String {
    Json::object(vec![
        ("jsonrpc", Json::str("2.0")),
        ("id", id.clone()),
        ("result", result),
    ])
    .to_json()
}

fn error_response(id: &Json, code: i32, message: &str) -> String {
    Json::object(vec![
        ("jsonrpc", Json::str("2.0")),
        ("id", id.clone()),
        (
            "error",
            Json::object(vec![
                ("code", Json::Number(f64::from(code))),
                ("message", Json::str(message)),
            ]),
        ),
    ])
    .to_json()
}

fn initialize(id: &Json) -> String {
    ok_response(
        id,
        Json::object(vec![
            ("protocolVersion", Json::str(PROTOCOL_VERSION)),
            (
                "capabilities",
                Json::object(vec![("tools", Json::object(vec![]))]),
            ),
            (
                "serverInfo",
                Json::object(vec![
                    ("name", Json::str("oxml-mcp")),
                    ("version", Json::str(env!("CARGO_PKG_VERSION"))),
                ]),
            ),
        ]),
    )
}

fn tool(
    name: &str,
    description: &str,
    props: &[(&str, &str, &str)],
    required: &[&str],
) -> Json {
    let properties: Vec<(&str, Json)> = props
        .iter()
        .map(|(n, ty, d)| {
            (
                *n,
                Json::object(vec![
                    ("type", Json::str(*ty)),
                    ("description", Json::str(*d)),
                ]),
            )
        })
        .collect();
    Json::object(vec![
        ("name", Json::str(name)),
        ("description", Json::str(description)),
        (
            "inputSchema",
            Json::object(vec![
                ("type", Json::str("object")),
                ("properties", Json::object(properties)),
                (
                    "required",
                    Json::Array(
                        required.iter().map(|r| Json::str(*r)).collect(),
                    ),
                ),
            ]),
        ),
    ])
}

fn tools_list(id: &Json) -> String {
    ok_response(
        id,
        Json::object(vec![(
            "tools",
            Json::Array(vec![
                tool(
                    "xml_query",
                    "Evaluate an XPath 1.0 expression against an XML \
                     document and return the matching values. Use this \
                     instead of reading a large document into context.",
                    &[
                        ("xml", "string", "The XML document"),
                        ("xpath", "string", "An XPath 1.0 expression"),
                        (
                            "namespaces",
                            "object",
                            "Namespace prefixes used in the expression, \
                             mapping prefix to URI, e.g. \
                             {\"m\": \"urn:example\"}. A prefix must be \
                             bound here; it is not read from the \
                             document. Call xml_inspect to see which \
                             namespaces a document uses.",
                        ),
                    ],
                    &["xml", "xpath"],
                ),
                tool(
                    "xml_validate",
                    "Validate an XML document against an XML Schema \
                     (XSD). Returns every violation with the path to \
                     the element it concerns.",
                    &[
                        ("xml", "string", "The XML document"),
                        ("xsd", "string", "The XML Schema"),
                    ],
                    &["xml", "xsd"],
                ),
                tool(
                    "xml_check",
                    "Check whether a document is well-formed, and \
                     report the line and column if it is not.",
                    &[("xml", "string", "The XML document")],
                    &["xml"],
                ),
                tool(
                    "xml_inspect",
                    "Summarise a document's structure: element counts, \
                     depth, the element names present, and the \
                     namespaces it uses. Use this to understand a \
                     document's shape before querying it.",
                    &[("xml", "string", "The XML document")],
                    &["xml"],
                ),
            ]),
        )]),
    )
}

fn text_result(text: String, is_error: bool) -> Json {
    Json::object(vec![
        (
            "content",
            Json::Array(vec![Json::object(vec![
                ("type", Json::str("text")),
                ("text", Json::String(text)),
            ])]),
        ),
        ("isError", Json::Bool(is_error)),
    ])
}

fn tools_call(id: &Json, request: &Json) -> String {
    let Some(params) = request.get("params") else {
        return error_response(id, -32602, "missing params");
    };
    let Some(name) = params.get("name").and_then(Json::as_str) else {
        return error_response(id, -32602, "missing tool name");
    };
    let args = params.get("arguments").cloned().unwrap_or(Json::Null);
    let arg = |k: &str| args.get(k).and_then(Json::as_str).unwrap_or("");

    let result = match name {
        "xml_query" => {
            let namespaces = namespace_bindings(request);
            run_query(arg("xml"), arg("xpath"), &namespaces)
        }
        "xml_validate" => run_validate(arg("xml"), arg("xsd")),
        "xml_check" => run_check(arg("xml")),
        "xml_inspect" => run_inspect(arg("xml")),
        other => {
            return error_response(
                id,
                -32602,
                &format!("unknown tool `{other}`"),
            );
        }
    };

    ok_response(
        id,
        match result {
            Ok(text) => text_result(text, false),
            Err(text) => text_result(text, true),
        },
    )
}

fn parse_doc(xml: &str) -> Result<oxml::Document, String> {
    oxml::parse(xml).map_err(|e| {
        let (line, col) = e.line_column(xml);
        format!(
            "The document is not well-formed at line {line}, column {col}: {e}"
        )
    })
}

/// The `namespaces` argument of a `tools/call` request.
///
/// A JSON object mapping prefix to URI. Absent is the same as empty,
/// so a request that needs no namespaces is unchanged.
fn namespace_bindings(request: &Json) -> Vec<(String, String)> {
    let Some(Json::Object(object)) = request
        .get("params")
        .and_then(|p| p.get("arguments"))
        .and_then(|a| a.get("namespaces"))
    else {
        return Vec::new();
    };
    object
        .iter()
        .filter_map(|(prefix, value)| {
            // `xml` is bound by the specification; rebinding it is not
            // something a caller may do, so it is ignored rather than
            // failing a request over it.
            if prefix == "xml" {
                return None;
            }
            Some((prefix.clone(), value.as_str()?.to_owned()))
        })
        .collect()
}

fn run_query(
    xml: &str,
    xpath: &str,
    namespaces: &[(String, String)],
) -> Result<String, String> {
    let doc = parse_doc(xml)?;
    let bindings: Vec<(&str, &str)> = namespaces
        .iter()
        .map(|(prefix, uri)| (prefix.as_str(), uri.as_str()))
        .collect();
    let compiled = oxml::XPath::compile_with_namespaces(xpath, &bindings)
        .map_err(|e| {
            // The library names a Rust function, which is no use to a
            // model. Say what it can put in the request.
            if e.message.contains("unbound namespace prefix") {
                let prefix =
                    e.message.split('`').nth(1).unwrap_or("PREFIX").to_owned();
                format!(
                    "The XPath expression uses the namespace prefix \
                     `{prefix}`, which is not bound. Pass it in the \
                     `namespaces` argument, for example \
                     {{\"{prefix}\": \"urn:example\"}}. Call `xml_inspect` \
                     to see which namespaces the document uses."
                )
            } else {
                let (line, column) = xpath_line_column(xpath, e.offset);
                format!(
                    "The XPath expression is invalid at line {line}, column {column}: {}",
                    e.message
                )
            }
    })?;
    let value = compiled.evaluate(&doc);

    let Some(nodes) = value.nodes() else {
        return Ok(value.to_str(&doc));
    };
    if nodes.is_empty() {
        return Ok("No nodes matched.".to_owned());
    }
    let lines: Vec<String> = nodes
        .iter()
        .map(|n| doc.text(*n))
        .filter(|t| !t.trim().is_empty())
        .collect();
    Ok(if lines.is_empty() {
        format!("{} node(s) matched, all with empty text.", nodes.len())
    } else {
        lines.join("\n")
    })
}

fn xpath_line_column(input: &str, offset: usize) -> (usize, usize) {
    let mut end = offset.min(input.len());
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    let upto = &input[..end];
    let line = upto.matches('\n').count() + 1;
    let column = upto
        .rsplit('\n')
        .next()
        .map_or(1, |line| line.chars().count() + 1);
    (line, column)
}

fn run_validate(xml: &str, xsd: &str) -> Result<String, String> {
    let schema = xmlschema::parse_schema(xsd)
        .map_err(|e| format!("The schema could not be read: {e}"))?;
    let doc = parse_doc(xml)?;
    let report = xmlschema::validate(&doc, &schema);
    if report.is_valid() {
        return Ok("The document is valid against the schema.".to_owned());
    }
    let mut out = format!("{} violation(s):\n", report.violations.len());
    for v in &report.violations {
        let _ = writeln!(out, "  {} — {}", v.path, v.message);
    }
    Err(out)
}

fn run_check(xml: &str) -> Result<String, String> {
    let doc = parse_doc(xml)?;
    Ok(format!(
        "The document is well-formed ({} nodes).",
        doc.len()
    ))
}

fn run_inspect(xml: &str) -> Result<String, String> {
    use std::collections::BTreeMap;
    let doc = parse_doc(xml)?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut namespaces: BTreeMap<String, usize> = BTreeMap::new();
    let mut depth_max = 0usize;

    for id in doc.descendants() {
        if let Some(name) = doc.element_name(id) {
            *counts.entry(name.local.clone()).or_default() += 1;
            if let Some(uri) = &name.namespace {
                *namespaces.entry(uri.clone()).or_default() += 1;
            }
            let mut d = 0usize;
            let mut cur = Some(id);
            while let Some(n) = cur {
                cur = doc.parent(n);
                d += 1;
            }
            depth_max = depth_max.max(d);
        }
    }

    let root = doc
        .root_element()
        .and_then(|r| doc.element_name(r))
        .map_or_else(|| "none".to_owned(), |n| n.local.clone());

    let mut out = format!(
        "Root element: {root}\nMaximum depth: {depth_max}\nElements:\n"
    );
    for (name, n) in counts {
        let _ = writeln!(out, "  {name}: {n}");
    }
    // A model cannot write a namespace-aware query against namespaces
    // it cannot see, and from oxml 0.0.4 an unbound prefix is an error
    // rather than a silent match. Reporting them here is what makes the
    // `namespaces` argument usable.
    if namespaces.is_empty() {
        let _ = writeln!(out, "Namespaces: none");
    } else {
        let _ = writeln!(
            out,
            "Namespaces (pass these to xml_query as `namespaces`):"
        );
        for (uri, n) in namespaces {
            let _ = writeln!(out, "  {uri}: {n} element(s)");
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "<library><book lang=\"en\"><title>Dune</title>\
                       </book><book lang=\"fr\"><title>Germinal</title>\
                       </book></library>";

    /// Drive the server exactly as a client does: one line in, at most
    /// one line out, both parsed as JSON.
    fn call(line: &str) -> Option<Json> {
        let out = handle_line(line)?;
        assert!(!out.contains('\n'), "response spans lines: {out}");
        Some(json::parse(&out).expect("response is valid JSON"))
    }

    fn request(id: i32, method: &str, params: Json) -> String {
        Json::object(vec![
            ("jsonrpc", Json::str("2.0")),
            ("id", Json::Number(f64::from(id))),
            ("method", Json::str(method)),
            ("params", params),
        ])
        .to_json()
    }

    fn tool_call(tool: &str, args: Vec<(&str, Json)>) -> Json {
        call(&request(
            1,
            "tools/call",
            Json::object(vec![
                ("name", Json::str(tool)),
                ("arguments", Json::object(args)),
            ]),
        ))
        .expect("a call has an id, so it has a response")
    }

    fn text_of(response: &Json) -> &str {
        let Some(Json::Array(content)) =
            response.get("result").and_then(|r| r.get("content"))
        else {
            panic!("no content in {}", response.to_json());
        };
        content[0].get("text").and_then(Json::as_str).expect("text")
    }

    fn is_error(response: &Json) -> bool {
        matches!(
            response.get("result").and_then(|r| r.get("isError")),
            Some(Json::Bool(true))
        )
    }

    #[test]
    fn initialize_reports_the_protocol_version() {
        let r = call(&request(1, "initialize", Json::Null)).expect("reply");
        assert_eq!(r.get("jsonrpc").and_then(Json::as_str), Some("2.0"));
        assert_eq!(
            r.get("result")
                .and_then(|x| x.get("protocolVersion"))
                .and_then(Json::as_str),
            Some("2024-11-05")
        );
    }

    #[test]
    fn the_response_id_matches_the_request_id() {
        // Clients correlate on this; a mismatch hangs the caller.
        let r = call(&request(42, "initialize", Json::Null)).expect("reply");
        assert_eq!(r.get("id"), Some(&Json::Number(42.0)));

        let s = handle_line(
            r#"{"jsonrpc":"2.0","id":"abc","method":"initialize"}"#,
        )
        .expect("reply");
        let s = json::parse(&s).expect("valid");
        assert_eq!(s.get("id").and_then(Json::as_str), Some("abc"));
    }

    #[test]
    fn tools_list_advertises_every_implemented_tool() {
        let r = call(&request(1, "tools/list", Json::Null)).expect("reply");
        let Some(Json::Array(tools)) =
            r.get("result").and_then(|x| x.get("tools"))
        else {
            panic!("no tools array");
        };
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name")?.as_str())
            .collect();
        assert_eq!(
            names,
            ["xml_query", "xml_validate", "xml_check", "xml_inspect"]
        );
        // An advertised tool with no schema is unusable to a client.
        for t in tools {
            assert!(t.get("description").is_some());
            let schema = t.get("inputSchema").expect("inputSchema");
            assert_eq!(
                schema.get("type").and_then(Json::as_str),
                Some("object")
            );
            assert!(schema.get("properties").is_some());
            assert!(schema.get("required").is_some());
        }
    }

    #[test]
    fn every_advertised_tool_is_callable() {
        // Guards the pairing between `tools_list` and `tools_call`:
        // the two lists are written out separately.
        for tool in ["xml_query", "xml_validate", "xml_check", "xml_inspect"] {
            let r = tool_call(tool, vec![("xml", Json::str(DOC))]);
            assert!(
                r.get("result").is_some(),
                "{tool} was advertised but rejected: {}",
                r.to_json()
            );
        }
    }

    #[test]
    fn xml_query_returns_the_selected_text() {
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(DOC)),
                ("xpath", Json::str("//book[1]/title")),
            ],
        );
        assert!(!is_error(&r));
        assert!(text_of(&r).contains("Dune"), "{}", text_of(&r));
    }

    #[test]
    fn xml_query_reads_attributes() {
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(DOC)),
                ("xpath", Json::str("//book[2]/@lang")),
            ],
        );
        assert!(!is_error(&r));
        assert!(text_of(&r).contains("fr"), "{}", text_of(&r));
    }

    #[test]
    fn xml_check_accepts_and_rejects() {
        let good = tool_call("xml_check", vec![("xml", Json::str(DOC))]);
        assert!(!is_error(&good));

        let bad =
            tool_call("xml_check", vec![("xml", Json::str("<a><b></a>"))]);
        assert!(is_error(&bad), "{}", bad.to_json());
        // The position is what makes the message actionable.
        assert!(text_of(&bad).contains(':'), "{}", text_of(&bad));
    }

    #[test]
    fn xml_inspect_summarises_the_document() {
        let r = tool_call("xml_inspect", vec![("xml", Json::str(DOC))]);
        let text = text_of(&r);
        assert!(text.contains("library"), "{text}");
        assert!(text.contains("book: 2"), "{text}");
    }

    #[test]
    fn xml_validate_reports_both_outcomes() {
        let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:element name="note" type="xs:string"/>
        </xs:schema>"#;
        let ok = tool_call(
            "xml_validate",
            vec![
                ("xml", Json::str("<note>hi</note>")),
                ("xsd", Json::str(xsd)),
            ],
        );
        assert!(!is_error(&ok), "{}", ok.to_json());

        let bad = tool_call(
            "xml_validate",
            vec![
                ("xml", Json::str("<wrong>hi</wrong>")),
                ("xsd", Json::str(xsd)),
            ],
        );
        assert!(is_error(&bad), "{}", bad.to_json());
    }

    #[test]
    fn a_tool_failure_is_a_result_not_a_protocol_error() {
        // MCP distinguishes the two: a bad *document* must come back as
        // isError content so the model can read and react to it, not as
        // a JSON-RPC error that the client surfaces as a transport fault.
        let r = tool_call("xml_check", vec![("xml", Json::str("<a>"))]);
        assert!(r.get("error").is_none(), "{}", r.to_json());
        assert!(is_error(&r));
    }

    #[test]
    fn notifications_get_no_reply() {
        // Replying to a notification is a protocol violation.
        assert!(
            handle_line(r#"{"jsonrpc":"2.0","method":"initialize"}"#).is_none()
        );
        assert!(
            handle_line(
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
            )
            .is_none()
        );
    }

    #[test]
    fn malformed_input_gets_a_parse_error_not_a_panic() {
        let r = call("not json at all").expect("reply");
        assert_eq!(
            r.get("error").and_then(|e| e.get("code")),
            Some(&Json::Number(-32700.0))
        );
        // A parse error has no id to echo, so it must be null.
        assert_eq!(r.get("id"), Some(&Json::Null));
    }

    #[test]
    fn a_request_without_a_method_is_an_invalid_request() {
        // It has an id, so the client is waiting for a response.
        let r = call(r#"{"jsonrpc":"2.0","id":7}"#).expect("reply");
        assert_eq!(
            r.get("error").and_then(|e| e.get("code")),
            Some(&Json::Number(-32600.0))
        );
        assert_eq!(r.get("id"), Some(&Json::Number(7.0)));
    }

    #[test]
    fn unknown_methods_and_tools_are_rejected_distinctly() {
        let m = call(&request(1, "no/such", Json::Null)).expect("reply");
        assert_eq!(
            m.get("error").and_then(|e| e.get("code")),
            Some(&Json::Number(-32601.0))
        );

        let t = tool_call("no_such_tool", vec![]);
        assert_eq!(
            t.get("error").and_then(|e| e.get("code")),
            Some(&Json::Number(-32602.0))
        );
    }

    #[test]
    fn missing_params_and_arguments_do_not_panic() {
        let r = call(&request(1, "tools/call", Json::Null));
        assert!(r.expect("reply").get("error").is_some());

        // `arguments` omitted entirely: every argument reads as empty.
        let r = call(&request(
            1,
            "tools/call",
            Json::object(vec![("name", Json::str("xml_check"))]),
        ))
        .expect("reply");
        assert!(is_error(&r), "{}", r.to_json());
    }

    #[test]
    fn control_characters_in_a_document_survive_the_round_trip() {
        // An unescaped control character would corrupt the whole line,
        // and this is the layer that decides.
        //
        // U+0001 is not a legal XML character, so the document is
        // rejected -- correctly, and that is not what this test is
        // about. What matters is that the *reply* comes back as one
        // well-formed line with nothing raw in it, whichever way the
        // parse went.
        let r = tool_call(
            "xml_check",
            vec![("xml", Json::str("<a>\u{1}\u{7}</a>"))],
        );
        let line = r.to_json();
        assert!(!line.contains('\n'), "reply must be one line");
        assert!(
            !line.chars().any(char::is_control),
            "no raw control character may reach the transport: {line:?}"
        );
        assert!(crate::json::parse(&line).is_ok(), "reply must parse");

        // A tab *is* legal, and must survive escaped rather than
        // splitting or corrupting the line.
        let r = tool_call("xml_check", vec![("xml", Json::str("<a>\t</a>"))]);
        assert!(!is_error(&r), "{}", r.to_json());
    }

    #[test]
    fn a_bound_prefix_selects_only_that_namespace() {
        // From oxml 0.0.4 a prefix resolves against bindings supplied
        // with the query, not against the document.
        let xml =
            r#"<r xmlns:m="urn:u"><m:item>ns</m:item><item>plain</item></r>"#;
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(xml)),
                ("xpath", Json::str("//m:item")),
                ("namespaces", Json::object(vec![("m", Json::str("urn:u"))])),
            ],
        );
        assert!(!is_error(&r), "{}", r.to_json());
        assert!(r.to_json().contains("ns"), "{}", r.to_json());
    }

    #[test]
    fn an_unbound_prefix_says_what_to_do_about_it() {
        // The library's message names a Rust function, which is no use
        // to a model. The reply must name the argument to pass and the
        // tool that reveals what to put in it.
        let xml = r#"<r xmlns:m="urn:u"><m:item>ns</m:item></r>"#;
        let r = tool_call(
            "xml_query",
            vec![("xml", Json::str(xml)), ("xpath", Json::str("//m:item"))],
        );
        assert!(is_error(&r));
        let text = r.to_json();
        assert!(text.contains("namespaces"), "{text}");
        assert!(text.contains("xml_inspect"), "{text}");
    }

    #[test]
    fn inspect_reports_the_namespaces_a_document_uses() {
        // A model cannot write a namespace-aware query against
        // namespaces it cannot see.
        let xml = r#"<r xmlns:m="urn:u"><m:item>ns</m:item></r>"#;
        let r = tool_call("xml_inspect", vec![("xml", Json::str(xml))]);
        assert!(r.to_json().contains("urn:u"), "{}", r.to_json());

        let plain = tool_call(
            "xml_inspect",
            vec![("xml", Json::str("<r><item/></r>"))],
        );
        assert!(
            plain.to_json().contains("Namespaces: none"),
            "{}",
            plain.to_json()
        );
    }

    #[test]
    fn the_xml_prefix_may_not_be_rebound() {
        // Bound by the specification; a binding that tries is ignored
        // rather than failing the request.
        let xml = r#"<r><a xml:lang="en">x</a></r>"#;
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(xml)),
                ("xpath", Json::str("//@xml:lang")),
                (
                    "namespaces",
                    Json::object(vec![("xml", Json::str("urn:wrong"))]),
                ),
            ],
        );
        assert!(!is_error(&r), "{}", r.to_json());
        assert!(r.to_json().contains("en"), "{}", r.to_json());
    }

    #[test]
    fn responses_are_always_a_single_line() {
        // The transport is line-delimited: an embedded newline splits
        // one response into two unparseable halves.
        let r = handle_line(&request(
            1,
            "tools/call",
            Json::object(vec![
                ("name", Json::str("xml_check")),
                (
                    "arguments",
                    Json::object(vec![(
                        "xml",
                        Json::str("<a>\nline\ntwo\n</a>"),
                    )]),
                ),
            ]),
        ))
        .expect("reply");
        assert!(!r.contains('\n'), "{r}");
    }

    #[test]
    fn a_scalar_expression_returns_its_value() {
        // Not a node-set: the value is the answer, and returning an
        // empty match here would be wrong.
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(DOC)),
                ("xpath", Json::str("count(//book)")),
            ],
        );
        assert!(!is_error(&r));
        assert_eq!(text_of(&r).trim(), "2");
    }

    #[test]
    fn a_query_matching_nothing_says_so() {
        // An empty string would read to the model as a successful query
        // against an empty document.
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(DOC)),
                ("xpath", Json::str("//nonexistent")),
            ],
        );
        assert!(!is_error(&r));
        assert!(text_of(&r).contains("No nodes matched"), "{}", text_of(&r));
    }

    #[test]
    fn matches_with_no_text_report_the_count_instead() {
        // Empty elements match but have nothing to show; silence would
        // be indistinguishable from no match at all.
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str("<r><e/><e/></r>")),
                ("xpath", Json::str("//e")),
            ],
        );
        assert!(!is_error(&r));
        let text = text_of(&r);
        assert!(text.contains('2'), "{text}");
        assert!(text.contains("empty text"), "{text}");
    }

    #[test]
    fn an_invalid_xpath_is_reported_as_such() {
        let r = tool_call(
            "xml_query",
            vec![("xml", Json::str(DOC)), ("xpath", Json::str("//["))],
        );
        assert!(is_error(&r));
        assert!(
            text_of(&r)
                .contains("XPath expression is invalid at line 1, column 3"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn an_invalid_multiline_xpath_reports_expression_line_and_column() {
        let r = tool_call(
            "xml_query",
            vec![
                ("xml", Json::str(DOC)),
                ("xpath", Json::str("//book[\n@lang = ]")),
            ],
        );
        assert!(is_error(&r));
        assert!(
            text_of(&r)
                .contains("XPath expression is invalid at line 2, column 9"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn a_call_without_params_is_an_invalid_params_error() {
        let r = call(&request(1, "tools/call", Json::Null)).expect("reply");
        assert_eq!(
            r.get("error").and_then(|e| e.get("code")),
            Some(&Json::Number(-32602.0))
        );
    }

    #[test]
    fn a_call_without_a_tool_name_is_rejected() {
        let r = call(&request(
            1,
            "tools/call",
            Json::object(vec![("arguments", Json::object(vec![]))]),
        ))
        .expect("reply");
        assert_eq!(
            r.get("error").and_then(|e| e.get("code")),
            Some(&Json::Number(-32602.0))
        );
    }
}
