mod compact;
mod normalized;
mod shared;
mod unsafe_content;

use serde_json::Value;

use crate::{diagnostic::ValidationReport, options::ValidationOptions};

#[must_use]
pub fn validate_report(value: &Value) -> ValidationReport {
    validate_report_with_options(value, &ValidationOptions::default())
}

#[must_use]
pub fn validate_report_with_options(
    value: &Value,
    options: &ValidationOptions,
) -> ValidationReport {
    compact::validate_compact_report(value, &options.limits)
}

#[must_use]
pub fn validate_normalized_report(value: &Value) -> ValidationReport {
    validate_normalized_report_with_options(value, &ValidationOptions::default())
}

#[must_use]
pub fn validate_normalized_report_with_options(
    value: &Value,
    options: &ValidationOptions,
) -> ValidationReport {
    normalized::validate_normalized_report_with_options(value, options)
}
