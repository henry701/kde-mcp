//! Reading `audit.jsonl` into [`AuditRecord`]s.

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

/// The five timestamps the daemon records around a tool call (nanoseconds).
/// Mirrors the Rust `AuditRecord.timings` field in `src/audit.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct AuditTimings {
    pub t_received_ns: i64,
    pub t_policy_done_ns: i64,
    pub t_impl_start_ns: i64,
    pub t_impl_done_ns: i64,
    pub t_audit_written_ns: i64,
}

/// One audited tool call. `timings` is absent for calls that failed before the
/// implementation stage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuditRecord {
    pub tool: String,
    /// ISO-8601 wall-clock written by the daemon (the `ts` field, not `timestamp`).
    pub ts: String,
    #[serde(default)]
    pub timings: Option<AuditTimings>,
}

/// Read a JSONL audit log, one [`AuditRecord`] per non-empty line.
pub fn parse_jsonl(path: &Path) -> io::Result<Vec<AuditRecord>> {
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rec: AuditRecord = serde_json::from_str(trimmed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        out.push(rec);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.jsonl")
    }

    #[test]
    fn yields_every_line_as_a_record() {
        let records = parse_jsonl(&fixture()).unwrap();
        assert_eq!(records.len(), 4);
    }

    #[test]
    fn preserves_the_tool_name() {
        let records = parse_jsonl(&fixture()).unwrap();
        assert_eq!(records[0].tool, "observe");
        assert_eq!(records[2].tool, "screenshot");
    }

    #[test]
    fn parses_timings_when_present() {
        let records = parse_jsonl(&fixture()).unwrap();
        assert_eq!(records[0].timings.unwrap().t_impl_done_ns, 45_000_000);
    }

    #[test]
    fn leaves_timings_none_when_absent() {
        let records = parse_jsonl(&fixture()).unwrap();
        assert!(records[3].timings.is_none());
    }
}
