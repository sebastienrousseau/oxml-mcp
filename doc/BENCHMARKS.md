<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Benchmarks

```bash
cargo bench --bench protocol
```

## What is being measured, and why

An MCP client sends one request and waits for one reply, so the figure
that matters is **latency per call**, not throughput. The benchmark
reports it per method.

The question the benchmark exists to answer is how much of that latency
belongs to *this* crate rather than to `oxml`. `initialize` and
`tools/list` do no XML work at all, so they price the JSON-RPC layer on
its own; the `xml_*` calls price it plus the parse.

## The comparison has to be paired

The first version of this benchmark timed `handle_line` and
`oxml::parse` in separate loops and divided. It reported
`xml_validate` as **faster than the parse it contains** — an impossible
result, and a clear sign that consecutive runs on a busy machine
disagree by more than the effect being measured. Whichever loop
happened to land in a quieter moment won.

The overhead is now measured by alternating the two inside one loop, so
both meet the same conditions. Four runs of the paired form gave +8.0%,
+12.0%, +15.5% and +25.1%: still a wide spread on a loaded machine, but
never negative. Read it as *roughly 10–25% on a 200 KB payload*, and
expect a tighter figure on a quiet one.

That cost is JSON-RPC unescaping a 200 KB XML document out of a JSON
string literal and escaping the reply back into one. It is proportional
to payload size, so it barely registers on the small documents a model
actually sends most of the time — a 1 KB `xml_check` is tens of
microseconds in total.

## Indicative figures

From one run on an Apple Silicon laptop that was **not** idle. These
describe the machine as much as the code; compare runs, not numbers.

| Request | Time | Line |
|---|---:|---:|
| `initialize` | 2.1 µs | 60 B |
| `tools/list` | 11.9 µs | 60 B |
| malformed line | 1.9 µs | 15 B |
| `xml_check`, 10 entries | 33.9 µs | 1,081 B |
| `xml_check`, 2,000 entries | ~3.0 ms | 200,821 B |

`tools/list` costs five times `initialize` because it serialises the
full schema for four tools every time it is asked. A client asks once
per session, so it has not been worth caching.

## Why this needs a library target

`serve` and `handle_line` are library functions. Before 0.0.7 the whole
server lived in `src/main.rs`, and a benchmark could only have driven it
as a subprocess — which measures process spawn, pipe setup and the OS
scheduler alongside the thing under test, at a scale that would bury a
25% difference entirely. The same extraction is what lets
`tests/serve.rs` hand the loop a writer that fails on demand.
