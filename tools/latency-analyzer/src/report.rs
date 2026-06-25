//! Rendering aggregated stats as a markdown report.

use crate::aggregate::ToolStats;

/// Labels for the report header.
pub struct RenderOptions {
    /// e.g. `"7 days"` or `"all time"`.
    pub since_label: String,
    /// e.g. `"2026-05-19"`.
    pub ending_label: String,
}

/// Render the two-table markdown report: totals (sorted by p95 descending) and
/// per-phase p95 (in first-seen order).
pub fn render_report(stats: &[ToolStats], opts: &RenderOptions) -> String {
    let total_n: usize = stats.iter().map(|s| s.n).sum();
    let n_tools = stats.len();
    let since = &opts.since_label;
    let ending = &opts.ending_label;

    let mut sorted: Vec<&ToolStats> = stats.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.total_p95_ms));
    let rows = sorted
        .iter()
        .map(|s| {
            format!(
                "| {:<14}| {:>4}| {:>6}| {:>6}| {:>6}| {}",
                s.tool, s.n, s.total_p50_ms, s.total_p95_ms, s.total_p99_ms, s.dominant_phase_p95
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let phase_rows = stats
        .iter()
        .map(|s| {
            let p = &s.phases_p95_ms;
            format!(
                "| {:<14}| {:>4}| {:>6}| {:>4}| {:>5}",
                s.tool, p.rmcp, p.policy, p.r#impl, p.audit
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "# kde-mcp latency profile — {since} ending {ending}\n\nN = {total_n} tool calls across {n_tools} tools.\n\n| Tool          |    n | p50 ms | p95 ms | p99 ms | dominant phase (p95)\n|---------------|-----:|-------:|-------:|-------:|---------------------\n{rows}\n\n## Phase-level p95 (ms)\n\n| Tool          | rmcp | policy | impl | audit\n|---------------|-----:|-------:|-----:|-----:\n{phase_rows}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::{Phases, ToolStats};

    fn stats() -> Vec<ToolStats> {
        vec![
            ToolStats {
                tool: "observe".to_string(),
                n: 312,
                total_p50_ms: 45,
                total_p95_ms: 98,
                total_p99_ms: 140,
                phases_p95_ms: Phases {
                    rmcp: 2,
                    policy: 1,
                    r#impl: 92,
                    audit: 3,
                },
                dominant_phase_p95: "impl".to_string(),
            },
            ToolStats {
                tool: "screenshot".to_string(),
                n: 87,
                total_p50_ms: 220,
                total_p95_ms: 480,
                total_p99_ms: 720,
                phases_p95_ms: Phases {
                    rmcp: 2,
                    policy: 1,
                    r#impl: 470,
                    audit: 5,
                },
                dominant_phase_p95: "impl".to_string(),
            },
        ]
    }

    fn opts() -> RenderOptions {
        RenderOptions {
            since_label: "7 days".to_string(),
            ending_label: "2026-05-19".to_string(),
        }
    }

    #[test]
    fn produces_the_expected_header() {
        let md = render_report(&stats(), &opts());
        assert!(md.contains("# kde-mcp latency profile — 7 days ending 2026-05-19"));
    }

    #[test]
    fn includes_each_tool_row() {
        let md = render_report(&stats(), &opts());
        assert!(md.contains("| observe       |  312|     45|     98|    140| impl"));
        assert!(md.contains("| screenshot    |   87|    220|    480|    720| impl"));
    }

    #[test]
    fn reports_total_n_across_all_tools() {
        let md = render_report(&stats(), &opts());
        assert!(md.contains("N = 399 tool calls across 2 tools"));
    }

    #[test]
    fn sorts_rows_by_p95_descending() {
        let md = render_report(&stats(), &opts());
        let obs = md.find("| observe").unwrap();
        let shot = md.find("| screenshot").unwrap();
        // screenshot (p95 480) sorts before observe (p95 98) in the totals table.
        assert!(shot < obs);
    }
}
