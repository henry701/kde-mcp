use std::process::ExitCode;

use chrono::Utc;
use kde_mcp_latency_analyzer::cli::{parse_args, report_for};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let now_ms = Utc::now().timestamp_millis();
    let ending = Utc::now().format("%Y-%m-%d").to_string();

    match report_for(&args.audit_path, args.since_ms, now_ms, &ending) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
