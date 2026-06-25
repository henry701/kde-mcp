//! Parse kde-mcp's `audit.jsonl`, aggregate per-tool latency percentiles, and
//! render a markdown report. The library is split so each stage is unit-tested
//! in isolation; `main.rs` wires them together behind a small CLI.

pub mod aggregate;
pub mod cli;
pub mod parse;
pub mod report;
