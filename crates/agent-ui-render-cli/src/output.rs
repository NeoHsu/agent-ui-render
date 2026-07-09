use agent_ui_render_core::{Finding, FindingLevel, ValidationReport};
use serde::Serialize;
use serde_json::json;

use crate::cli::OutputFormat;

pub fn print_findings(report: &ValidationReport, output: OutputFormat) {
    if output == OutputFormat::Json {
        let payload = json!({
            "valid": report.errors.is_empty(),
            "errors": report.errors,
            "warnings": report.warnings,
        });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
        );
        return;
    }
    for finding in &report.errors {
        eprintln!("ERROR: {}: {}", finding.path, finding.message);
    }
    for finding in &report.warnings {
        eprintln!("WARNING: {}: {}", finding.path, finding.message);
    }
}

pub fn print_extra_warnings(warnings: &[Finding], output: OutputFormat) {
    if warnings.is_empty() {
        return;
    }
    if output == OutputFormat::Json {
        let payload = json!({ "warnings": warnings });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
        );
        return;
    }
    for finding in warnings {
        let label = match finding.level {
            FindingLevel::Error => "ERROR",
            FindingLevel::Warning => "WARNING",
        };
        eprintln!("{label}: {}: {}", finding.path, finding.message);
    }
}

pub fn print_json<T: Serialize>(value: &T, pretty: bool) -> anyhow::Result<()> {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}
