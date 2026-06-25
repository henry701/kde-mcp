//! Aggregating records into per-tool percentile statistics.

use crate::parse::AuditRecord;

/// Per-phase p95 latencies (milliseconds), keyed by the delta they measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phases {
    /// `t_policy_done - t_received` (deserialise + policy enter).
    pub rmcp: i64,
    /// `t_impl_start - t_policy_done`.
    pub policy: i64,
    /// `t_impl_done - t_impl_start`.
    pub r#impl: i64,
    /// `t_audit_written - t_impl_done`.
    pub audit: i64,
}

/// Aggregated latency statistics for one tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStats {
    pub tool: String,
    pub n: usize,
    pub total_p50_ms: i64,
    pub total_p95_ms: i64,
    pub total_p99_ms: i64,
    pub phases_p95_ms: Phases,
    /// Name of the phase with the largest p95 (`rmcp` | `policy` | `impl` | `audit`).
    pub dominant_phase_p95: String,
}

/// Nearest-rank percentile on an unsorted sample. `p` in `[0, 100]`.
pub fn percentile(xs: &[i64], p: f64) -> i64 {
    if xs.is_empty() {
        return 0;
    }
    let mut sorted = xs.to_vec();
    sorted.sort_unstable();
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank - 1]
}

/// Floor-divide nanoseconds to milliseconds.
pub fn ns_to_ms(ns: i64) -> i64 {
    ns / 1_000_000
}

/// Bucket records by tool (skipping those without timings) and compute
/// per-tool percentiles. Returns tools in first-seen order.
pub fn aggregate(records: &[AuditRecord]) -> Vec<ToolStats> {
    let mut order: Vec<String> = Vec::new();
    let mut by_tool: std::collections::HashMap<String, Vec<&AuditRecord>> =
        std::collections::HashMap::new();
    for r in records {
        if r.timings.is_none() {
            continue;
        }
        if !by_tool.contains_key(&r.tool) {
            order.push(r.tool.clone());
        }
        by_tool.entry(r.tool.clone()).or_default().push(r);
    }

    let mut out = Vec::new();
    for tool in order {
        let rs = &by_tool[&tool];
        let delta = |f: &dyn Fn(&crate::parse::AuditTimings) -> i64| -> Vec<i64> {
            rs.iter().map(|r| f(&r.timings.unwrap())).collect()
        };
        let totals = delta(&|t| t.t_audit_written_ns);
        let rmcp = delta(&|t| t.t_policy_done_ns - t.t_received_ns);
        let policy = delta(&|t| t.t_impl_start_ns - t.t_policy_done_ns);
        let impl_ = delta(&|t| t.t_impl_done_ns - t.t_impl_start_ns);
        let audit = delta(&|t| t.t_audit_written_ns - t.t_impl_done_ns);

        let phases = Phases {
            rmcp: ns_to_ms(percentile(&rmcp, 95.0)),
            policy: ns_to_ms(percentile(&policy, 95.0)),
            r#impl: ns_to_ms(percentile(&impl_, 95.0)),
            audit: ns_to_ms(percentile(&audit, 95.0)),
        };

        // First phase with the strictly-largest p95 wins (rmcp/policy/impl/audit order).
        let mut dominant = "rmcp";
        let mut max = -1i64;
        for (name, value) in [
            ("rmcp", phases.rmcp),
            ("policy", phases.policy),
            ("impl", phases.r#impl),
            ("audit", phases.audit),
        ] {
            if value > max {
                max = value;
                dominant = name;
            }
        }

        out.push(ToolStats {
            tool: tool.clone(),
            n: rs.len(),
            total_p50_ms: ns_to_ms(percentile(&totals, 50.0)),
            total_p95_ms: ns_to_ms(percentile(&totals, 95.0)),
            total_p99_ms: ns_to_ms(percentile(&totals, 99.0)),
            phases_p95_ms: phases,
            dominant_phase_p95: dominant.to_string(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{AuditRecord, AuditTimings};

    fn fixture(tool: &str, impl_done_ns: i64) -> AuditRecord {
        AuditRecord {
            tool: tool.to_string(),
            ts: "2026-05-19T10:00:00Z".to_string(),
            timings: Some(AuditTimings {
                t_received_ns: 0,
                t_policy_done_ns: 1_000_000,
                t_impl_start_ns: 1_100_000,
                t_impl_done_ns: impl_done_ns,
                t_audit_written_ns: impl_done_ns + 500_000,
            }),
        }
    }

    fn get<'a>(stats: &'a [ToolStats], tool: &str) -> &'a ToolStats {
        stats.iter().find(|s| s.tool == tool).expect("tool present")
    }

    #[test]
    fn percentile_of_a_sorted_array() {
        let xs: Vec<i64> = (1..=10).collect();
        assert_eq!(percentile(&xs, 50.0), 5); // ceil(0.5 * 10) = 5
        assert_eq!(percentile(&xs, 95.0), 10); // ceil(0.95 * 10) = 10
        assert_eq!(percentile(&xs, 99.0), 10);
    }

    #[test]
    fn percentile_handles_a_single_value() {
        assert_eq!(percentile(&[42], 95.0), 42);
    }

    #[test]
    fn percentile_of_empty_is_zero() {
        assert_eq!(percentile(&[], 95.0), 0);
    }

    #[test]
    fn aggregate_buckets_records_by_tool_and_emits_percentiles() {
        let records = vec![
            fixture("observe", 10_000_000),
            fixture("observe", 20_000_000),
            fixture("observe", 30_000_000),
            fixture("observe", 100_000_000),
            fixture("screenshot", 200_000_000),
        ];
        let result = aggregate(&records);
        let observe = get(&result, "observe");
        assert_eq!(observe.n, 4);
        // p50 of [10, 20, 30, 100] ms (nearest-rank): ceil(0.5*4) = 2 -> 20 ms
        assert_eq!(observe.total_p50_ms, 20);
        // p95 (ceil(0.95*4) = 4) -> 100 ms
        assert_eq!(observe.total_p95_ms, 100);

        let ss = get(&result, "screenshot");
        assert_eq!(ss.n, 1);
    }

    #[test]
    fn aggregate_skips_records_without_timings() {
        let records = vec![
            fixture("observe", 10_000_000),
            AuditRecord {
                tool: "observe".to_string(),
                ts: "2026-05-19T10:00:00Z".to_string(),
                timings: None,
            },
        ];
        let result = aggregate(&records);
        assert_eq!(get(&result, "observe").n, 1);
    }
}
