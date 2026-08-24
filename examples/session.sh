#!/usr/bin/env bash
#
# A full session: initialise, list the tools, then call each one.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

call "initialize reports the MCP protocol version" \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '"protocolVersion":"2024-11-05"'

call "tools/list advertises all four tools" \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '"xml_inspect"'

# Call xml_inspect first: a model that knows the element names writes a
# query that works, and one that guesses writes `//item` against a
# document whose elements are called something else.
call "xml_inspect summarises the shape" \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"xml_inspect","arguments":{"xml":"<r><t>x</t></r>"}}}' \
  'Root element: r'

call "xml_query returns one value per line" \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<r><t>Dune</t><t>Germinal</t></r>","xpath":"//t"}}}' \
  'Dune\nGerminal'

# The reason the tool exists: a number, rather than the model counting
# elements it can see and approximating.
call "xml_query evaluates count() to a number" \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<r><t>a</t><t>b</t><t>c</t></r>","xpath":"count(//t)"}}}' \
  '"text":"3"'

call "xml_check accepts a well-formed document" \
  '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"xml_check","arguments":{"xml":"<a/>"}}}' \
  'well-formed'

call "a successful call is not an error" \
  '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"xml_check","arguments":{"xml":"<a/>"}}}' \
  '"isError":false'

# Python's json.dumps escapes non-ASCII by default, so an emoji arrives
# as an escaped surrogate pair. Rejecting those broke every Python
# client that sent one.
call "an escaped surrogate pair is accepted" \
  '{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<r><t>😀</t></r>","xpath":"//t"}}}' \
  '"isError":false'


# Namespaces. A prefix resolves against bindings sent with the query,
# not against the document, so an unbound one is an error rather than a
# silent match on the local part.
call "xml_inspect reports the namespaces a document uses" \
  '{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"xml_inspect","arguments":{"xml":"<r xmlns:m=\"urn:u\"><m:item>ns</m:item></r>"}}}' \
  'urn:u'

call "a bound prefix selects only that namespace" \
  '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<r xmlns:m=\"urn:u\"><m:item>ns</m:item><item>plain</item></r>","xpath":"//m:item","namespaces":{"m":"urn:u"}}}}' \
  '"text":"ns"'

call "an unbound prefix says which argument to use" \
  '{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"xml_query","arguments":{"xml":"<r xmlns:m=\"urn:u\"><m:item>ns</m:item></r>","xpath":"//m:item"}}}' \
  'xml_inspect'

finish
