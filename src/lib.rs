// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! `oxml-mcp` — a Model Context Protocol server for XML.
//!
//! Four tools: query, validate, check, and inspect. The protocol is
//! handled by [`rmcp`], the official MCP SDK; this crate supplies the
//! tools and the text a model reads.
//!
//! Why a model wants this: an LLM asked to pull a value out of a large
//! XML document otherwise has to read the whole thing into its
//! context and pattern-match by eye. An `XPath` tool turns that into a
//! question with an exact answer, and the document never needs to fit
//! in the context window.
//!
//! The four operations are plain functions -- [`query`], [`validate`],
//! [`check`], [`inspect`] -- and [`XmlServer`] is the handler that
//! exposes them as MCP tools. Each answer is returned twice: as text
//! for the model, and as a structured value for a client that wants to
//! read it without parsing prose.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::{ToolCallContext, schema_for_output};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
    ErrorData, Implementation, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A parse failure, worded for a model.
///
/// The location is what makes the message actionable: a model told
/// only that the document is malformed will guess at where.
fn parse_doc(xml: &str) -> Result<oxml::Document, String> {
    oxml::parse(xml).map_err(|e| {
        let (line, col) = e.line_column(xml);
        format!(
            "The document is not well-formed at line {line}, column {col}: {e}"
        )
    })
}

/// What an `XPath` expression selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct QueryOutput {
    /// How many nodes matched. A scalar expression counts as one.
    pub count: usize,
    /// The matched values in document order, empty text omitted. A
    /// scalar expression -- a number, string or boolean -- is one
    /// value.
    pub values: Vec<String>,
}

impl fmt::Display for QueryOutput {
    /// One value per line. When nothing matched, say so: an empty
    /// string would read to a model as a successful query against an
    /// empty document.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count == 0 {
            return f.write_str("No nodes matched.");
        }
        if self.values.is_empty() {
            return write!(
                f,
                "{} node(s) matched, all with empty text.",
                self.count
            );
        }
        f.write_str(&self.values.join("\n"))
    }
}

/// Evaluate an `XPath` 1.0 expression against a document.
///
/// `namespaces` binds the prefixes the expression uses; a prefix is
/// never read from the document. The `xml` prefix is bound by the
/// specification and a binding for it is ignored rather than refused.
///
/// # Errors
///
/// A document that is not well-formed, or an expression that does not
/// compile, is reported as text a model can act on: the position of
/// the fault, or the argument to pass for an unbound prefix.
pub fn query(
    xml: &str,
    xpath: &str,
    namespaces: &[(&str, &str)],
) -> Result<QueryOutput, String> {
    let doc = parse_doc(xml)?;
    let bindings: Vec<(&str, &str)> = namespaces
        .iter()
        .copied()
        .filter(|(prefix, _)| *prefix != "xml")
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
        return Ok(QueryOutput {
            count: 1,
            values: vec![value.to_str(&doc)],
        });
    };
    let values: Vec<String> = nodes
        .iter()
        .map(|n| doc.text(*n))
        .filter(|t| !t.trim().is_empty())
        .collect();
    Ok(QueryOutput {
        count: nodes.len(),
        values,
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

/// One schema violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Violation {
    /// The path to the element the violation concerns.
    pub path: String,
    /// What is wrong with it.
    pub message: String,
}

/// The outcome of validating a document against a schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ValidateOutput {
    /// Whether the document conforms to the schema.
    pub valid: bool,
    /// Every violation found; empty when the document is valid.
    pub violations: Vec<Violation>,
}

impl fmt::Display for ValidateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.valid {
            return f.write_str("The document is valid against the schema.");
        }
        writeln!(f, "{} violation(s):", self.violations.len())?;
        for v in &self.violations {
            writeln!(f, "  {} — {}", v.path, v.message)?;
        }
        Ok(())
    }
}

/// Validate a document against an XML Schema.
///
/// A document that violates the schema is a successful validation with
/// `valid: false`; the violations are the answer.
///
/// # Errors
///
/// The schema could not be read, or the document is not well-formed.
/// Neither is a validation result, because nothing was validated.
pub fn validate(xml: &str, xsd: &str) -> Result<ValidateOutput, String> {
    let schema = xmlschema::parse_schema(xsd)
        .map_err(|e| format!("The schema could not be read: {e}"))?;
    let doc = parse_doc(xml)?;
    let report = xmlschema::validate(&doc, &schema);
    Ok(ValidateOutput {
        valid: report.is_valid(),
        violations: report
            .violations
            .iter()
            .map(|v| Violation {
                path: v.path.clone(),
                message: v.message.clone(),
            })
            .collect(),
    })
}

/// A well-formed document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CheckOutput {
    /// Always true: a document that is not well-formed is an error,
    /// not a result.
    pub well_formed: bool,
    /// How many nodes the document has.
    pub nodes: usize,
}

impl fmt::Display for CheckOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The document is well-formed ({} nodes).", self.nodes)
    }
}

/// Check whether a document is well-formed.
///
/// # Errors
///
/// The document is not, and the message says where.
pub fn check(xml: &str) -> Result<CheckOutput, String> {
    let doc = parse_doc(xml)?;
    Ok(CheckOutput {
        well_formed: true,
        nodes: doc.len(),
    })
}

/// The shape of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct InspectOutput {
    /// The local name of the root element, or `none`.
    pub root: String,
    /// The deepest element, counting the root as 1.
    pub max_depth: usize,
    /// Every element name present, with how many times it occurs.
    pub elements: BTreeMap<String, usize>,
    /// Every namespace URI in use, with how many elements are in it.
    pub namespaces: BTreeMap<String, usize>,
}

impl fmt::Display for InspectOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Root element: {}", self.root)?;
        writeln!(f, "Maximum depth: {}", self.max_depth)?;
        writeln!(f, "Elements:")?;
        for (name, n) in &self.elements {
            writeln!(f, "  {name}: {n}")?;
        }
        // A model cannot write a namespace-aware query against
        // namespaces it cannot see, and an unbound prefix is an error
        // rather than a silent match. Reporting them here is what
        // makes the `namespaces` argument usable.
        if self.namespaces.is_empty() {
            writeln!(f, "Namespaces: none")
        } else {
            writeln!(
                f,
                "Namespaces (pass these to xml_query as `namespaces`):"
            )?;
            for (uri, n) in &self.namespaces {
                writeln!(f, "  {uri}: {n} element(s)")?;
            }
            Ok(())
        }
    }
}

/// Summarise a document's structure.
///
/// # Errors
///
/// The document is not well-formed.
pub fn inspect(xml: &str) -> Result<InspectOutput, String> {
    let doc = parse_doc(xml)?;
    let mut elements: BTreeMap<String, usize> = BTreeMap::new();
    let mut namespaces: BTreeMap<String, usize> = BTreeMap::new();
    let mut max_depth = 0usize;

    for id in doc.descendants() {
        if let Some(name) = doc.element_name(id) {
            *elements.entry(name.local.clone()).or_default() += 1;
            if let Some(uri) = &name.namespace {
                *namespaces.entry(uri.clone()).or_default() += 1;
            }
            let mut d = 0usize;
            let mut cur = Some(id);
            while let Some(n) = cur {
                cur = doc.parent(n);
                d += 1;
            }
            max_depth = max_depth.max(d);
        }
    }

    let root = doc
        .root_element()
        .and_then(|r| doc.element_name(r))
        .map_or_else(|| "none".to_owned(), |n| n.local.clone());

    Ok(InspectOutput {
        root,
        max_depth,
        elements,
        namespaces,
    })
}

// The doc comments on the argument structs are the descriptions a
// client shows the model, kept word for word from the previous
// release; backticks would change them. The examples are what an
// auditor or a client with no document of its own sends: a string that
// happens to be XML rather than one that happens not to be.

/// Arguments of `xml_query`.
#[allow(clippy::doc_markdown, reason = "tool descriptions, shown verbatim")]
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryArgs {
    /// The XML document
    #[schemars(example = &"<library><book lang=\"en\"><title>Dune</title></book></library>")]
    pub xml: String,
    /// An XPath 1.0 expression
    #[schemars(example = &"//book/title")]
    pub xpath: String,
    /// Namespace prefixes used in the expression, mapping prefix to
    /// URI, e.g. {"m": "urn:example"}. A prefix must be bound here; it
    /// is not read from the document. Call xml_inspect to see which
    /// namespaces a document uses.
    #[serde(default)]
    pub namespaces: BTreeMap<String, String>,
}

/// Arguments of `xml_validate`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateArgs {
    /// The XML document
    #[schemars(example = &"<library><book lang=\"en\"><title>Dune</title></book></library>")]
    pub xml: String,
    /// The XML Schema
    #[schemars(example = &"<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"><xs:element name=\"library\"/></xs:schema>")]
    pub xsd: String,
}

/// Arguments of `xml_check` and `xml_inspect`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentArgs {
    /// The XML document
    #[schemars(example = &"<library><book lang=\"en\"><title>Dune</title></book></library>")]
    pub xml: String,
}

/// A tool result carrying the same answer twice: as text for the
/// model and as a structured value for the client.
///
/// A failure keeps the text only. The structured schema describes a
/// result, and an error is not one.
fn reply<T: Serialize + fmt::Display>(
    outcome: Result<T, String>,
    is_error: impl FnOnce(&T) -> bool,
) -> Result<CallToolResult, ErrorData> {
    match outcome {
        Ok(value) => {
            let structured = serde_json::to_value(&value)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            let content = vec![ContentBlock::text(value.to_string())];
            let mut result = if is_error(&value) {
                CallToolResult::error(content)
            } else {
                CallToolResult::success(content)
            };
            result.structured_content = Some(structured);
            Ok(result)
        }
        // A tool that ran and could not do the job: a *successful*
        // JSON-RPC response carrying `isError`, so the model sees the
        // text and can react to it. A JSON-RPC error would be handled
        // by the client and never shown.
        Err(message) => {
            Ok(CallToolResult::error(vec![ContentBlock::text(message)]))
        }
    }
}

/// The MCP server: the four tools over [`rmcp`].
///
/// Cheap to create and to clone; the HTTP transports create one per
/// session. It holds no document between calls.
#[derive(Debug, Clone)]
pub struct XmlServer {
    tool_router: ToolRouter<Self>,
}

impl Default for XmlServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
#[allow(
    clippy::unused_self,
    reason = "the SDK's tool router calls tools as methods"
)]
impl XmlServer {
    /// A server with all four tools registered.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "xml_query",
        description = "Evaluate an XPath 1.0 expression against an XML \
                       document and return the matching values. Use this \
                       instead of reading a large document into context.",
        annotations(
            title = "Query XML with XPath",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        ),
        output_schema = schema_for_output::<QueryOutput>()
    )]
    fn xml_query(
        &self,
        Parameters(args): Parameters<QueryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let namespaces: Vec<(&str, &str)> = args
            .namespaces
            .iter()
            .map(|(p, u)| (p.as_str(), u.as_str()))
            .collect();
        reply(query(&args.xml, &args.xpath, &namespaces), |_| false)
    }

    #[tool(
        name = "xml_validate",
        description = "Validate an XML document against an XML Schema \
                       (XSD). Returns every violation with the path to \
                       the element it concerns.",
        annotations(
            title = "Validate XML against an XSD",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        ),
        output_schema = schema_for_output::<ValidateOutput>()
    )]
    fn xml_validate(
        &self,
        Parameters(args): Parameters<ValidateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        // A violation is a tool failure the model must see, so it is
        // flagged `isError` -- but it is also a complete validation
        // result, so the structured half is kept.
        reply(validate(&args.xml, &args.xsd), |v| !v.valid)
    }

    #[tool(
        name = "xml_check",
        description = "Check whether a document is well-formed, and \
                       report the line and column if it is not.",
        annotations(
            title = "Check XML well-formedness",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        ),
        output_schema = schema_for_output::<CheckOutput>()
    )]
    fn xml_check(
        &self,
        Parameters(args): Parameters<DocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        reply(check(&args.xml), |_| false)
    }

    #[tool(
        name = "xml_inspect",
        description = "Summarise a document's structure: element counts, \
                       depth, the element names present, and the \
                       namespaces it uses. Use this to understand a \
                       document's shape before querying it.",
        annotations(
            title = "Inspect XML structure",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        ),
        output_schema = schema_for_output::<InspectOutput>()
    )]
    fn xml_inspect(
        &self,
        Parameters(args): Parameters<DocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        reply(inspect(&args.xml), |_| false)
    }
}

#[tool_handler(router = self.tool_router)]
#[allow(
    clippy::unused_async_trait_impl,
    reason = "the SDK's handler macro generates the trait methods"
)]
impl ServerHandler for XmlServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("oxml-mcp", env!("CARGO_PKG_VERSION"))
                    .with_title("oxml MCP")
                    .with_website_url(env!("CARGO_PKG_REPOSITORY")),
            )
            .with_instructions(
                "XML tools. Documents are passed as strings, never as \
                 paths. xml_inspect reports a document's element names \
                 and namespaces; xml_query evaluates XPath 1.0 against \
                 it; xml_check reports well-formedness; xml_validate \
                 checks it against an XSD.",
            )
    }

    /// A tool the server does not have is reported as a tool result,
    /// not a protocol error.
    ///
    /// The SDK's default is `-32602`, which the stateless HTTP revision
    /// carries as an HTTP 400 -- a transport fault to the client, and
    /// nothing a model gets to read. A model that misspelt a tool name
    /// is better served by text saying so.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if !self.tool_router.has_route(&request.name) {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                format!(
                    "Unknown tool: {}. The tools are xml_query, xml_validate, \
                 xml_check and xml_inspect.",
                    request.name
                ),
            )])
            .into());
        }
        let call = ToolCallContext::new(self, request, context);
        self.tool_router.call(call).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const DOC: &str = "<library><book lang=\"en\"><title>Dune</title>\
                       </book><book lang=\"fr\"><title>Germinal</title>\
                       </book></library>";

    /// Call a tool the way a request reaches it: JSON arguments,
    /// deserialised into the tool's parameter type.
    fn parse<T: serde::de::DeserializeOwned>(v: Value) -> T {
        serde_json::from_value(v).expect("arguments")
    }

    fn call(tool: &str, args: Value) -> CallToolResult {
        let server = XmlServer::new();
        let result = match tool {
            "xml_query" => server.xml_query(Parameters(parse(args))),
            "xml_validate" => server.xml_validate(Parameters(parse(args))),
            "xml_check" => server.xml_check(Parameters(parse(args))),
            "xml_inspect" => server.xml_inspect(Parameters(parse(args))),
            other => panic!("no such tool {other}"),
        };
        result.expect("a tool failure is a result, not a protocol error")
    }

    fn text_of(r: &CallToolResult) -> &str {
        r.content
            .first()
            .and_then(ContentBlock::as_text)
            .map(|t| t.text.as_str())
            .expect("text content")
    }

    fn is_error(r: &CallToolResult) -> bool {
        r.is_error == Some(true)
    }

    #[test]
    fn every_tool_is_registered_with_schema_and_annotations() {
        let tools = XmlServer::tool_router().list_all();
        let mut names: Vec<&str> =
            tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["xml_check", "xml_inspect", "xml_query", "xml_validate"]
        );
        for t in &tools {
            assert!(t.description.is_some(), "{} has no description", t.name);
            assert_eq!(
                t.input_schema.get("type").and_then(Value::as_str),
                Some("object")
            );
            assert!(t.input_schema.get("properties").is_some());
            assert!(
                t.output_schema.is_some(),
                "{} has no outputSchema",
                t.name
            );
            let a = t.annotations.as_ref().expect("annotations");
            assert_eq!(a.read_only_hint, Some(true), "{}", t.name);
        }
    }

    #[test]
    fn input_schemas_keep_the_field_names_and_descriptions() {
        let tools = XmlServer::tool_router().list_all();
        let query =
            tools.iter().find(|t| t.name == "xml_query").expect("query");
        let props = query.input_schema.get("properties").expect("properties");
        assert_eq!(
            props["xml"]["description"].as_str(),
            Some("The XML document")
        );
        assert_eq!(
            props["xpath"]["description"].as_str(),
            Some("An XPath 1.0 expression")
        );
        assert!(
            props["namespaces"]["description"]
                .as_str()
                .is_some_and(|d| d.contains("xml_inspect")),
            "{props}"
        );
        assert_eq!(props["namespaces"]["type"].as_str(), Some("object"));
        let required =
            query.input_schema["required"].as_array().expect("required");
        assert_eq!(required, &[json!("xml"), json!("xpath")]);
    }

    #[test]
    fn xml_query_returns_the_selected_text() {
        let r =
            call("xml_query", json!({"xml": DOC, "xpath": "//book[1]/title"}));
        assert!(!is_error(&r));
        assert!(text_of(&r).contains("Dune"), "{}", text_of(&r));
        assert_eq!(
            r.structured_content,
            Some(json!({"count": 1, "values": ["Dune"]}))
        );
    }

    #[test]
    fn xml_query_reads_attributes() {
        let r =
            call("xml_query", json!({"xml": DOC, "xpath": "//book[2]/@lang"}));
        assert!(!is_error(&r));
        assert!(text_of(&r).contains("fr"), "{}", text_of(&r));
    }

    #[test]
    fn xml_check_accepts_and_rejects() {
        let good = call("xml_check", json!({"xml": DOC}));
        assert!(!is_error(&good));
        assert_eq!(
            good.structured_content,
            Some(json!({"well_formed": true, "nodes": 11}))
        );

        let bad = call("xml_check", json!({"xml": "<a><b></a>"}));
        assert!(is_error(&bad));
        // The position is what makes the message actionable.
        assert!(
            text_of(&bad).contains("line 1, column"),
            "{}",
            text_of(&bad)
        );
        assert!(bad.structured_content.is_none());
    }

    #[test]
    fn xml_inspect_summarises_the_document() {
        let r = call("xml_inspect", json!({"xml": DOC}));
        let text = text_of(&r);
        assert!(text.contains("Root element: library"), "{text}");
        assert!(text.contains("book: 2"), "{text}");
        let s = r.structured_content.expect("structured");
        assert_eq!(s["root"], "library");
        assert_eq!(s["max_depth"], 4);
        assert_eq!(s["elements"]["book"], 2);
    }

    #[test]
    fn xml_validate_reports_both_outcomes() {
        let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:element name="note" type="xs:string"/>
        </xs:schema>"#;
        let ok = call(
            "xml_validate",
            json!({"xml": "<note>hi</note>", "xsd": xsd}),
        );
        assert!(!is_error(&ok), "{ok:?}");
        assert_eq!(
            ok.structured_content,
            Some(json!({"valid": true, "violations": []}))
        );

        let bad = call(
            "xml_validate",
            json!({"xml": "<wrong>hi</wrong>", "xsd": xsd}),
        );
        assert!(is_error(&bad), "{bad:?}");
        assert!(text_of(&bad).contains("violation(s)"), "{}", text_of(&bad));
        // A violation is still a complete validation result.
        let s = bad.structured_content.expect("structured");
        assert_eq!(s["valid"], false);
        assert!(!s["violations"].as_array().expect("array").is_empty());

        let unreadable =
            call("xml_validate", json!({"xml": "<a/>", "xsd": "<"}));
        assert!(is_error(&unreadable));
        assert!(text_of(&unreadable).contains("schema could not be read"));
    }

    #[test]
    fn a_tool_failure_is_a_result_not_a_protocol_error() {
        // MCP distinguishes the two: a bad *document* must come back as
        // isError content so the model can read and react to it, not as
        // a JSON-RPC error that the client surfaces as a transport fault.
        let r = XmlServer::new().xml_check(Parameters(DocumentArgs {
            xml: "<a>".to_owned(),
        }));
        let r = r.expect("Ok, not Err");
        assert!(is_error(&r));
    }

    #[test]
    fn a_bound_prefix_selects_only_that_namespace() {
        // A prefix resolves against bindings supplied with the query,
        // not against the document.
        let xml =
            r#"<r xmlns:m="urn:u"><m:item>ns</m:item><item>plain</item></r>"#;
        let r = call(
            "xml_query",
            json!({"xml": xml, "xpath": "//m:item", "namespaces": {"m": "urn:u"}}),
        );
        assert!(!is_error(&r), "{r:?}");
        assert_eq!(text_of(&r), "ns");
    }

    #[test]
    fn an_unbound_prefix_says_what_to_do_about_it() {
        // The library's message names a Rust function, which is no use
        // to a model. The reply must name the argument to pass and the
        // tool that reveals what to put in it.
        let xml = r#"<r xmlns:m="urn:u"><m:item>ns</m:item></r>"#;
        let r = call("xml_query", json!({"xml": xml, "xpath": "//m:item"}));
        assert!(is_error(&r));
        let text = text_of(&r);
        assert!(text.contains("namespaces"), "{text}");
        assert!(text.contains("xml_inspect"), "{text}");
    }

    #[test]
    fn inspect_reports_the_namespaces_a_document_uses() {
        let xml = r#"<r xmlns:m="urn:u"><m:item>ns</m:item></r>"#;
        let r = call("xml_inspect", json!({"xml": xml}));
        assert!(text_of(&r).contains("urn:u"), "{}", text_of(&r));
        assert_eq!(
            r.structured_content.expect("structured")["namespaces"]["urn:u"],
            1
        );

        let plain = call("xml_inspect", json!({"xml": "<r><item/></r>"}));
        assert!(
            text_of(&plain).contains("Namespaces: none"),
            "{}",
            text_of(&plain)
        );
    }

    #[test]
    fn the_xml_prefix_may_not_be_rebound() {
        // Bound by the specification; a binding that tries is ignored
        // rather than failing the request.
        let xml = r#"<r><a xml:lang="en">x</a></r>"#;
        let r = call(
            "xml_query",
            json!({"xml": xml, "xpath": "//@xml:lang", "namespaces": {"xml": "urn:wrong"}}),
        );
        assert!(!is_error(&r), "{r:?}");
        assert_eq!(text_of(&r), "en");
    }

    #[test]
    fn a_scalar_expression_returns_its_value() {
        // Not a node-set: the value is the answer, and returning an
        // empty match here would be wrong.
        let r =
            call("xml_query", json!({"xml": DOC, "xpath": "count(//book)"}));
        assert!(!is_error(&r));
        assert_eq!(text_of(&r).trim(), "2");
        assert_eq!(
            r.structured_content,
            Some(json!({"count": 1, "values": ["2"]}))
        );
    }

    #[test]
    fn a_query_matching_nothing_says_so() {
        let r =
            call("xml_query", json!({"xml": DOC, "xpath": "//nonexistent"}));
        assert!(!is_error(&r));
        assert!(text_of(&r).contains("No nodes matched"), "{}", text_of(&r));
        assert_eq!(
            r.structured_content,
            Some(json!({"count": 0, "values": []}))
        );
    }

    #[test]
    fn matches_with_no_text_report_the_count_instead() {
        // Empty elements match but have nothing to show; silence would
        // be indistinguishable from no match at all.
        let r = call(
            "xml_query",
            json!({"xml": "<r><e/><e/></r>", "xpath": "//e"}),
        );
        assert!(!is_error(&r));
        let text = text_of(&r);
        assert!(text.contains('2'), "{text}");
        assert!(text.contains("empty text"), "{text}");
        assert_eq!(
            r.structured_content,
            Some(json!({"count": 2, "values": []}))
        );
    }

    #[test]
    fn an_invalid_xpath_is_reported_as_such() {
        let r = call("xml_query", json!({"xml": DOC, "xpath": "//["}));
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
        let r = call(
            "xml_query",
            json!({"xml": DOC, "xpath": "//book[\n@lang = ]"}),
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
    fn xpath_positions_stop_at_character_boundaries() {
        // An offset inside a multi-byte character must not slice the
        // string mid-character.
        assert_eq!(xpath_line_column("é", 1), (1, 1));
        assert_eq!(xpath_line_column("ab", 9), (1, 3));
    }

    #[test]
    fn a_control_character_is_rejected_with_a_location() {
        // U+0001 is not a legal XML character; the reply must still be
        // a readable message, not a panic.
        let r = call("xml_check", json!({"xml": "<a>\u{1}</a>"}));
        assert!(is_error(&r));
        assert!(text_of(&r).contains("line 1"), "{}", text_of(&r));
        // A tab *is* legal.
        let r = call("xml_check", json!({"xml": "<a>\t</a>"}));
        assert!(!is_error(&r), "{r:?}");
    }

    #[test]
    fn the_server_describes_itself() {
        let info = XmlServer::default().get_info();
        assert_eq!(info.server_info.name, "oxml-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());
    }
}
