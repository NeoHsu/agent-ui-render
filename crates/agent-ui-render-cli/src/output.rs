use std::io::{self, Write};

use agent_ui_render_core::{Finding, FindingLevel, ValidationReport};
use serde::Serialize;
use serde_json::json;

use crate::cli::OutputFormat;

#[derive(Debug, thiserror::Error)]
#[error("failed to write stdout")]
pub struct StdoutWriteError {
    #[source]
    source: io::Error,
}

impl StdoutWriteError {
    pub fn is_broken_pipe(&self) -> bool {
        self.source.kind() == io::ErrorKind::BrokenPipe
    }
}

pub fn print_validation_result(
    report: &ValidationReport,
    output: OutputFormat,
) -> anyhow::Result<()> {
    if output == OutputFormat::Json {
        write_stdout_line(&validation_json(report))?;
    } else {
        print_findings(report, output)?;
    }
    Ok(())
}

pub fn print_findings(report: &ValidationReport, output: OutputFormat) -> anyhow::Result<()> {
    if output == OutputFormat::Json {
        write_stderr_line(&validation_json(report))?;
        return Ok(());
    }
    for finding in &report.errors {
        print_human_message(&format!("ERROR: {}: {}", finding.path, finding.message))?;
    }
    for finding in &report.warnings {
        print_human_message(&format!("WARNING: {}: {}", finding.path, finding.message))?;
    }
    Ok(())
}

pub fn print_extra_warnings(warnings: &[Finding], output: OutputFormat) -> anyhow::Result<()> {
    if warnings.is_empty() {
        return Ok(());
    }
    if output == OutputFormat::Json {
        let payload = json!({ "warnings": warnings });
        write_stderr_line(
            &serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string()),
        )?;
        return Ok(());
    }
    for finding in warnings {
        let label = match finding.level {
            FindingLevel::Error => "ERROR",
            FindingLevel::Warning => "WARNING",
        };
        print_human_message(&format!("{label}: {}: {}", finding.path, finding.message))?;
    }
    Ok(())
}

fn validation_json(report: &ValidationReport) -> String {
    let payload = json!({
        "valid": report.errors.is_empty(),
        "errors": report.errors,
        "warnings": report.warnings,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
}

pub fn print_json<T: Serialize>(value: &T, pretty: bool) -> anyhow::Result<()> {
    let output = if pretty {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    write_stdout_line(&output)?;
    Ok(())
}

pub fn write_stdout(value: &str) -> Result<(), StdoutWriteError> {
    write_stdout_bytes(value.as_bytes())
}

pub fn write_stdout_bytes(value: &[u8]) -> Result<(), StdoutWriteError> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(value)
        .and_then(|()| stdout.flush())
        .map_err(|source| StdoutWriteError { source })
}

pub fn write_stdout_line(value: &str) -> Result<(), StdoutWriteError> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{value}")
        .and_then(|()| stdout.flush())
        .map_err(|source| StdoutWriteError { source })
}

pub fn print_human_message(value: &str) -> io::Result<()> {
    write_stderr_line(&terminal_safe(value))
}

pub fn write_stderr_line(value: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{value}")?;
    stderr.flush()
}

fn terminal_safe(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if is_terminal_control(character) {
            output.extend(character.escape_default());
        } else {
            output.push(character);
        }
    }
    output
}

fn is_terminal_control(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

#[cfg(test)]
mod tests {
    use super::terminal_safe;

    #[test]
    fn terminal_text_escapes_controls_and_bidi_overrides() {
        assert_eq!(
            terminal_safe("safe\n\u{1b}[31m\u{202e}txt"),
            r"safe\n\u{1b}[31m\u{202e}txt"
        );
    }
}
