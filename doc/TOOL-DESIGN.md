<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Tool design

## Four tools, not fourteen

Every tool's name, description and JSON schema sits in the model's
context on **every** request, whether or not it is used. A server with
twenty narrow tools spends more context describing itself than a small
document would occupy.

Four broad tools cover the ground because XPath is already a query
language. `xml_query` with `count(//record[@status="failed"])` does not
need a `count_failed_records` tool, and inventing one would be
describing XPath badly in English.

The test for adding a fifth: could a model achieve it with an
expression it can write? If yes, it is not a tool.

## `xml_inspect` exists so queries are not guesses

A model handed a document it cannot see will guess at element names,
write `//item` against a document whose elements are called `record`,
get nothing back, and conclude the document is empty.

`xml_inspect` returns the root element, the maximum depth, and every
element name with its count. It is the cheapest possible orientation,
and the tool description says to call it first.

## Documents are strings, not paths

The server never opens a file.

A tool taking a path would be a way for any model with access to this
server to read any file the server process can. The decision about
what a model may read belongs to the client, which already has a
permission model, a user, and a UI to ask in.

The cost is real: the document crosses the boundary on every call, and
for a large one that is wasteful. There is no state between calls, so
there is no cache to invalidate and nothing to leak between sessions —
which is also why a handle-based flow is not there yet.

Meanwhile, `xml_inspect` once and one precise `xml_query` beats several
exploratory ones.

## Output is text, not JSON

Tool results are plain text: one value per line for `xml_query`, a
short report for `xml_inspect`.

Returning JSON would mean the model parses a structure to read values
it was going to read as text anyway, and doubles the token cost of
every result through quoting and escaping.

## No dependencies

JSON parsing and serialisation are in `src/json.rs`, roughly 300 lines.

The entire input to this program is untrusted JSON arriving on stdin.
A dependency tree there is a liability, and the crate has none beyond
`oxml` and `xmlschema`.

Writing it did produce one bug worth recording: escaped surrogate pairs
were rejected. Python's `json.dumps` escapes non-ASCII by default, so
an emoji arrives as a surrogate pair and **every Python client sending
one failed**. It is fixed, and asserted in
[`examples/session.sh`](../examples/session.sh).
