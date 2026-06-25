# kde-mcp latency analyzer

Reads `~/.local/state/kde-mcp/audit.jsonl` and emits a markdown report with
p50/p95/p99 per tool, broken into the phases captured by the daemon (see
`src/audit.rs::AuditTimings`). A small standalone Rust binary — no external
runtime.

## Build & run

    cd tools/latency-analyzer
    cargo run --release -- --since 7d > ../../docs/latency-profile.md

Flags:
- `--since DURATION` — only consider records younger than DURATION (`1h`, `7d`,
  `30d`), or `all`. Default: all records.
- `--audit PATH` — override the default audit-log path
  (`~/.local/state/kde-mcp/audit.jsonl`).

## Test

    cargo test
