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

# A schema violation is a tool failure the model must see, and also a
# complete validation result, so both halves are returned.
call "a schema violation sets isError and keeps the structured result" \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"xml_validate","arguments":{"xml":"<wrong/>","xsd":"<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"><xs:element name=\"right\"/></xs:schema>"}}}' \
  '"valid":false'

# A tool the server does not have is reported as a tool result naming
# the tools it does have. The SDK's default, -32602, becomes an HTTP
# 400 in the stateless revision -- a transport fault the model never
# reads.
call "an unknown tool is a result naming the tools that exist" \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}' \
  'Unknown tool: no_such_tool'

# A missing argument is a tool failure the model can act on: the SDK
# names the field.
call "a missing required argument names the field" \
  '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<a/>"}}}' \
  'missing field `xpath`'

call "an unknown method is method-not-found" \
  '{"jsonrpc":"2.0","id":7,"method":"nope"}' \
  '-32601'

call "a request with an id but no method is an error" \
  '{"jsonrpc":"2.0","id":8}' \
  '"error"'

# A line that is not JSON at all draws no reply: the SDK skips it and
# reads the next line. The session survives it, which is the part that
# matters -- the request after it is answered.
call_raw "a line that is not JSON does not end the session" \
  "$INITIALIZE" "$INITIALIZED" '{not json' \
  '{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"xml_check","arguments":{"xml":"<a/>"}}}' \
  -- '"id":9'

# An external entity is never dereferenced, so a document that asks the
# server to read /etc/passwd gets nothing.
call "an external entity is never substituted" \
  '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<!DOCTYPE d [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><d>&x;</d>","xpath":"string(/d)"}}}' \
  '"isError"'

finish
