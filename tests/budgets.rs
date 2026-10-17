// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tool entry points must use the core's untrusted-input budgets.

#[test]
fn query_refuses_to_materialize_a_string_past_its_budget() {
    let source = format!(
        "<r>{}<n/>{}</r>",
        "x".repeat(512 * 1024),
        "x".repeat(512 * 1024 + 1)
    );
    let result = oxml_mcp::query(&source, "string(/r)", &[]);
    assert!(result.is_err(), "the model query ignored the string budget");
}

#[test]
fn query_bounds_the_cumulative_text_of_matched_nodes() {
    let source = format!("<r><n>{}</n></r>", "x".repeat(1024 * 1024));
    let error = oxml_mcp::query(&source, "/r | /r/n", &[])
        .map(|_| ())
        .expect_err("bounded output strings");
    assert!(error.contains("limit"), "{error}");
}

#[test]
fn check_refuses_a_document_past_its_source_budget() {
    let source = format!("<r>{}</r>", "x".repeat(8 * 1024 * 1024));
    assert!(oxml_mcp::check(&source).is_err());
}

#[test]
fn validation_refuses_a_schema_past_its_source_budget() {
    let schema = " ".repeat(8 * 1024 * 1024 + 1);
    let error =
        oxml_mcp::validate("<r/>", &schema).expect_err("bounded schema");
    assert!(error.contains("limit"), "{error}");
}
