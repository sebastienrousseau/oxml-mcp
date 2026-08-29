#![no_main]
//! An arbitrary line must never panic the server.
//!
//! This is the highest-value target in the crate. `handle_line` is fed
//! whatever a client sends, the client is usually a language model, and
//! the JSON parser underneath is hand-written rather than a dependency
//! that thousands of other projects have already fuzzed.
//!
//! The contract is total: any bytes at all produce a reply or `None`,
//! never a panic. A panic here takes down the session for every tool
//! call, not just the malformed one.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(line) = core::str::from_utf8(data) else {
        return;
    };

    if let Some(reply) = oxml_mcp::handle_line(line) {
        // A reply must be a single line of valid JSON. The transport
        // is newline-delimited, so a reply containing a newline
        // desynchronises the stream for everything after it -- a
        // failure that would show up as the *next* request being
        // misread, far from its cause.
        assert!(
            !reply.contains('\n'),
            "reply spans lines, which would desynchronise the stream"
        );
    }
});
