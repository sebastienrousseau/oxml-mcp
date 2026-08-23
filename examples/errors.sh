#!/usr/bin/env bash
#
# Failure, at both levels: a tool that could not do the job, and a
# request the protocol rejects. They are reported differently on
# purpose.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# A malformed document is a *tool* failure: a successful JSON-RPC
# response carrying isError, because the model has to see it to correct
# itself. A transport error is never shown to the model.
call "a malformed document sets isError" \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"xml_check","arguments":{"xml":"<a>"}}}' \
  '"isError":true'

call "a malformed document still returns a result, not an error" \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xml_check","arguments":{"xml":"<a>"}}}' \
  '"result"'

call "an invalid XPath expression sets isError" \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<a/>","xpath":"//["}}}' \
  '"isError":true'

# An unknown tool is a *protocol* error, not a tool failure: the tool
# never ran. MCP puts unknown tools and invalid arguments in the
# JSON-RPC error category and reserves isError for a tool that ran and
# could not do the job.
call "an unknown tool is a JSON-RPC error, not a tool failure" \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}' \
  '-32602'

# Protocol failures get JSON-RPC codes.
call "malformed JSON is a parse error" \
  '{not json' \
  '-32700'

call "an unknown method is method-not-found" \
  '{"jsonrpc":"2.0","id":5,"method":"nope"}' \
  '-32601'

call "a request with an id but no method is invalid" \
  '{"jsonrpc":"2.0","id":6}' \
  '-32600'

# An external entity is never dereferenced, so a document that asks the
# server to read /etc/passwd gets nothing.
call "an external entity is never substituted" \
  '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<!DOCTYPE d [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><d>&x;</d>","xpath":"string(/d)"}}}' \
  '"isError"'

finish
