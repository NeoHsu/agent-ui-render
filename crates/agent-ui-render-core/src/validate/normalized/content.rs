use serde_json::Value;

use crate::{diagnostic::ValidationReport, domain, options::Limits};

use super::super::shared::{
    is_alert_level, is_primitive, validate_count, validate_string_length, validate_unknown_fields,
};

pub(super) fn validate_normalized_metrics(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(metrics) = value.as_array() else {
        report.error("$.metrics", "field 'metrics' must be an array when present");
        return;
    };
    validate_count(
        metrics.len(),
        limits.max_metrics,
        "$.metrics",
        "metrics",
        report,
    );
    for (index, metric) in metrics.iter().enumerate() {
        let path = format!("$.metrics[{index}]");
        let Some(object) = metric.as_object() else {
            report.error(path, "metric must be an object");
            continue;
        };
        validate_unknown_fields(
            &path,
            object,
            &["label", "value", "format", "unit", "delta"],
            report,
        );
        if let Some(label) = object.get("label").and_then(Value::as_str) {
            validate_string_length(
                label,
                &format!("{path}.label"),
                limits.max_string_chars,
                "metric label",
                report,
            );
        } else {
            report.error(format!("{path}.label"), "metric label must be a string");
        }
        if let Some(value) = object.get("value") {
            if !is_primitive(value) {
                report.error(
                    format!("{path}.value"),
                    "metric value must be string, number, boolean, or null",
                );
            } else if let Some(text) = value.as_str() {
                validate_string_length(
                    text,
                    &format!("{path}.value"),
                    limits.max_string_chars,
                    "metric value",
                    report,
                );
            }
        } else {
            report.error(
                format!("{path}.value"),
                "metric value must be string, number, boolean, or null",
            );
        }
        if let Some(format) = object.get("format") {
            if let Some(format) = format.as_str() {
                validate_string_length(
                    format,
                    &format!("{path}.format"),
                    limits.max_string_chars,
                    "metric format",
                    report,
                );
                if !domain::METRIC_FORMATS.contains(&format) {
                    report.error(format!("{path}.format"), "metric format is unsupported");
                }
            } else {
                report.error(format!("{path}.format"), "metric format is unsupported");
            }
        }
    }
}

pub(super) fn validate_normalized_markdown(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(sections) = value.as_array() else {
        report.error(
            "$.markdown",
            "field 'markdown' must be an array when present",
        );
        return;
    };
    validate_count(
        sections.len(),
        limits.max_markdown_sections,
        "$.markdown",
        "markdown sections",
        report,
    );
    let mut total_markdown_chars = 0usize;
    for (index, section) in sections.iter().enumerate() {
        let path = format!("$.markdown[{index}]");
        let Some(object) = section.as_object() else {
            report.error(path, "markdown section must be an object");
            continue;
        };
        validate_unknown_fields(&path, object, &["title", "content"], report);
        if let Some(title) = object.get("title") {
            if let Some(title) = title.as_str() {
                validate_string_length(
                    title,
                    &format!("{path}.title"),
                    limits.max_string_chars,
                    "markdown title",
                    report,
                );
            } else {
                report.error(
                    format!("{path}.title"),
                    "markdown title must be a string when present",
                );
            }
        }
        if let Some(content) = object.get("content").and_then(Value::as_str) {
            total_markdown_chars = total_markdown_chars.saturating_add(content.chars().count());
            validate_string_length(
                content,
                &format!("{path}.content"),
                limits.max_markdown_section_chars,
                "markdown content",
                report,
            );
        } else {
            report.error(
                format!("{path}.content"),
                "markdown content must be a string",
            );
        }
    }
    if total_markdown_chars > limits.max_total_markdown_chars {
        report.error(
            "$.markdown",
            format!(
                "total markdown length {total_markdown_chars} chars exceeds max {}",
                limits.max_total_markdown_chars
            ),
        );
    }
}

pub(super) fn validate_normalized_alerts(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(alerts) = value.as_array() else {
        report.error("$.alerts", "field 'alerts' must be an array when present");
        return;
    };
    validate_count(
        alerts.len(),
        limits.max_alerts,
        "$.alerts",
        "alerts",
        report,
    );
    for (index, alert) in alerts.iter().enumerate() {
        let path = format!("$.alerts[{index}]");
        let Some(object) = alert.as_object() else {
            report.error(path, "alert must be an object");
            continue;
        };
        validate_unknown_fields(&path, object, &["level", "title", "content"], report);
        if let Some(level) = object.get("level").and_then(Value::as_str) {
            validate_string_length(
                level,
                &format!("{path}.level"),
                limits.max_string_chars,
                "alert level",
                report,
            );
            if !is_alert_level(level) {
                report.error(format!("{path}.level"), "alert level is unsupported");
            }
        } else {
            report.error(format!("{path}.level"), "alert level is unsupported");
        }
        if let Some(title) = object.get("title") {
            if let Some(title) = title.as_str() {
                validate_string_length(
                    title,
                    &format!("{path}.title"),
                    limits.max_string_chars,
                    "alert title",
                    report,
                );
            } else {
                report.error(
                    format!("{path}.title"),
                    "alert title must be a string when present",
                );
            }
        }
        if let Some(content) = object.get("content").and_then(Value::as_str) {
            validate_string_length(
                content,
                &format!("{path}.content"),
                limits.max_string_chars,
                "alert content",
                report,
            );
        } else {
            report.error(format!("{path}.content"), "alert content must be a string");
        }
    }
}
