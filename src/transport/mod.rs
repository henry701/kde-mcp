//! Wire-transport dispatch (ADR-0012).
//!
//! `Transport::Stdio` keeps the original one-client-per-process semantics
//! from before ADR-0012 — Claude Code spawns a kde-mcp child and speaks
//! over stdin/stdout. `Transport::Http` and `Transport::Unix` run the
//! singleton long-running daemon (HTTP on `127.0.0.1:7575`, or a Unix
//! domain socket), activated by systemd-user. All modes are first-class
//! peers and can run in parallel against the same on-disk state (lease,
//! approvals, audit log).

pub use crate::cli::Transport;

pub mod daemon;
pub mod session;
pub mod stdio;
