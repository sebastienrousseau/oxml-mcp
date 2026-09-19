#![no_main]
//! Arbitrary arguments must never panic a tool.
//!
//! The JSON-RPC layer now belongs to the SDK. What is still this
//! crate's own surface is the four tools, and what they are fed is
//! whatever a client sends: a document, an expression, a schema, each
//! usually written by a language model. The contract is total: any
//! strings at all produce a result or an error message, never a panic.
//! A panic takes down the session for every tool call, not just the
//! malformed one.
//!
//! The input is split on the first two NUL bytes into document,
//! expression and schema, so one corpus exercises every tool.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let mut parts = text.splitn(3, '\0');
    let xml = parts.next().unwrap_or_default();
    let xpath = parts.next().unwrap_or_default();
    let xsd = parts.next().unwrap_or_default();

    let _ = oxml_mcp::check(xml);
    let _ = oxml_mcp::inspect(xml);
    let _ = oxml_mcp::query(xml, xpath, &[("m", "urn:fuzz")]);
    let _ = oxml_mcp::validate(xml, xsd);
});
