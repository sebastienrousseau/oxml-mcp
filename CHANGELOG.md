# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.7] - 2026-08-28

### Changed

- Built on oxml 0.0.7 and xmlschema 0.0.7. The suite ships one version
  number across all six crates.

### Added

- **A library target.** The protocol handling that filled
  `src/main.rs` now lives in `src/lib.rs`; the binary supplies stdin
  and stdout and nothing else. `serve` is generic over its two ends,
  so a test or a benchmark can drive the real loop through an
  in-memory pipe.

- `benches/protocol.rs`, measuring latency per request. It exists to
  answer one question -- how much of a tool call is this crate rather
  than `oxml` -- and the answer is roughly 10-25% on a 200 KB payload,
  a few microseconds on a small one.

  The first version of that comparison timed the two in separate loops
  and reported `xml_validate` as *faster than the parse it contains*.
  Consecutive runs on a busy machine disagree by more than the effect,
  so the measurement is now paired: both alternate inside one loop.

- `tests/serve.rs`, covering what a real pipe will not produce on
  demand -- a writer that starts failing mid-session, input that stops
  mid-line, a malformed line followed by a good one.

- An **Examples** job in CI. README.md and doc/TESTING.md both said the
  examples ran there; nothing ran them.

### Fixed

- doc/TESTING.md quoted 48 tests and oxml's conformance as 2,394 of
  2,557. It is 57 and 2,557 of 2,557.

## [0.0.6] - 2026-08-26

### Changed

- Built on oxml 0.0.6 and xmlschema 0.0.6. The suite ships one version
  number across all six crates.

  xmlschema 0.0.6 is the substantial half of this release: its W3C
  conformance pass rate moved from 71.7% to 95.6%, and its coverage of
  the suite -- the share of tests that produce an answer meaning
  anything -- from 27.0% to 87.6%. Schemas this crate previously read
  as valid, and did not enforce, are now either enforced or reported
  as unenforceable.

## [0.0.5] - 2026-08-24

### Changed

- Built on oxml 0.0.5, which completes `XPath` 1.0: all thirteen axes
  and all 27 functions.

  **One behaviour change reaches expressions passed through this
  crate.** A function name outside the specification's library, or a
  call with the wrong number of arguments, used to compile and evaluate
  to an empty node-set. Both are now compile errors, reported with an
  offset. `starts-with("abc")` answered `true` before, because the
  absent argument read as the empty string.

  Six functions that previously answered `""` now work:
  `substring-before`, `substring-after`, `translate`, `name`, `id` and
  `lang`. So do the `following`, `preceding` and `namespace` axes.

## [0.0.4] - 2026-08-24

### Added

- `namespaces` on `xml_query`, and namespaces reported by
  `xml_inspect`. oxml 0.0.4 resolves prefixes in `XPath` name tests
  instead of matching on the local part alone, so a prefixed expression
  needs a binding.

## [0.0.3] - 2026-08-22

### Added

- Initial release. Model Context Protocol server exposing oxml's XML parsing, XPath and XSD validation
- Tracks the version line of the [`oxml`](https://github.com/sebastienrousseau/oxml)
  core, so a given version of any suite member is built and tested against
  the matching core.

[0.0.3]: https://github.com/sebastienrousseau/oxml-mcp/releases/tag/v0.0.3
