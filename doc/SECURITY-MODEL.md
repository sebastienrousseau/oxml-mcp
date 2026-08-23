<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Security model

Two hostile inputs, not one.

**The document**, because a model was asked to look at something from
the internet. **The JSON around it**, because that is the program's
entire input and it arrives on stdin from a process this one does not
control.

## The document

- **External entities are never dereferenced.** A document containing
  `<!ENTITY xxe SYSTEM "file:///etc/passwd">` cannot make the server
  read that file. There is no code that opens a file or a socket, so
  there is no option to get wrong. Asserted in
  [`examples/errors.sh`](../examples/errors.sh).
- **Entity expansion is bounded per document**, not per reference, so
  neither the exponential nor the quadratic blowup gets through.
- **Recursion is bounded**, so a deeply nested document returns an
  error rather than overflowing the stack — which would abort the
  process, and no caller can catch that.

Full reasoning:
<https://github.com/sebastienrousseau/oxml/blob/main/doc/SECURITY-MODEL.md>

## The JSON

`src/json.rs` is hand-written, about 300 lines, and has no
dependencies. For a program whose whole input is untrusted JSON, a
dependency tree is a liability.

Malformed JSON is answered with `-32700` and the server keeps reading.
It does not exit — a server that did could be killed by anything able
to write a byte to its stdin.

## What the server cannot do

- **Open a file.** Tools take document contents, never paths. A server
  that took paths would be a way to read any file the process can.
- **Make a network request.** No network code exists in the dependency
  tree.
- **Remember anything.** No state between calls, so nothing leaks
  between sessions and there is no cache to poison.

## Memory safety

`#![forbid(unsafe_code)]`, and no C dependency anywhere.

## What it does not protect you from

- **What the document says.** The server reports what is in the file.
  If a value is hostile to whatever the model does next, that is
  downstream of here.
- **A very large document.** It is parsed in full, in memory. A client
  that hands over a 2 GB document will find out.
- **Your own client's permissions.** This server does what it is asked
  with what it is given. What it is given is the client's decision.
