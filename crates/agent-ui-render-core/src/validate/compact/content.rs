use serde_json::Value;

use crate::{diagnostic::ValidationReport, options::Limits, wire::compact};

use super::super::shared::{
    is_alert_level_code, is_base_or_dict_type_code, is_primitive, validate_count,
    validate_string_length,
};

pub(super) fn validate_compact_metrics(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(metrics) = value.as_array() else {
        report.error("$.m", "field 'm' must be an array when present");
        return;
    };
    validate_count(metrics.len(), limits.max_metrics, "$.m", "metrics", report);
    for (index, metric) in metrics.iter().enumerate() {
        let path = format!("$.m[{index}]");
        let Some(tuple) = metric.as_array() else {
            report.error(path, "metric must be a tuple array");
            continue;
        };
        if !(2..=5).contains(&tuple.len()) {
            report.error(path.clone(), "metric tuple must have 2 to 5 entries");
        }
        if let Some(label) = tuple.first().and_then(Value::as_str) {
            validate_string_length(
                label,
                &format!("{path}[0]"),
                limits.max_string_chars,
                "metric label",
                report,
            );
        } else {
            report.error(format!("{path}[0]"), "metric label must be a string");
        }
        if let Some(value) = tuple.get(1) {
            if !is_primitive(value) {
                report.error(
                    format!("{path}[1]"),
                    "metric value must be string, number, boolean, or null",
                );
            } else if let Some(text) = value.as_str() {
                validate_string_length(
                    text,
                    &format!("{path}[1]"),
                    limits.max_string_chars,
                    "metric value",
                    report,
                );
            }
        } else {
            report.error(
                format!("{path}[1]"),
                "metric value must be string, number, boolean, or null",
            );
        }
        if let Some(format) = tuple.get(2).filter(|value| !value.is_null()) {
            if let Some(format) = format.as_str() {
                validate_string_length(
                    format,
                    &format!("{path}[2]"),
                    limits.max_string_chars,
                    "metric format",
                    report,
                );
                if !is_base_or_dict_type_code(format) {
                    report.error(
                        format!("{path}[2]"),
                        "metric format must be a supported type code",
                    );
                }
            } else {
                report.error(
                    format!("{path}[2]"),
                    "metric format must be a supported type code",
                );
            }
        }
        if let Some(unit) = tuple.get(3).filter(|value| !value.is_null()) {
            if let Some(unit) = unit.as_str() {
                validate_string_length(
                    unit,
                    &format!("{path}[3]"),
                    limits.max_string_chars,
                    "metric unit",
                    report,
                );
            } else {
                report.error(
                    format!("{path}[3]"),
                    "metric unit must be a string when present",
                );
            }
        }
        if let Some(delta) = tuple.get(4).filter(|value| !value.is_null()) {
            validate_compact_metric_delta(delta, &format!("{path}[4]"), report);
        }
    }
}

fn validate_compact_metric_delta(value: &Value, path: &str, report: &mut ValidationReport) {
    if value.is_number() {
        return;
    }
    let Some(tuple) = value.as_array() else {
        report.error(
            path.to_owned(),
            "metric delta must be a number or a [value, format] tuple",
        );
        return;
    };
    if tuple.is_empty() || tuple.len() > 2 {
        report.error(
            path.to_owned(),
            "metric delta tuple must have 1 or 2 entries",
        );
    }
    if !tuple.first().is_some_and(Value::is_number) {
        report.error(format!("{path}[0]"), "metric delta value must be a number");
    }
    if let Some(format) = tuple.get(1)
        && !format
            .as_str()
            .is_some_and(|code| compact::DELTA_FORMAT_CODES.contains(&code))
    {
        report.error(
            format!("{path}[1]"),
            "metric delta format must be 'n' or 'pct'",
        );
    }
}

pub(super) fn validate_compact_alerts(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(alerts) = value.as_array() else {
        report.error("$.a", "field 'a' must be an array when present");
        return;
    };
    validate_count(alerts.len(), limits.max_alerts, "$.a", "alerts", report);
    for (index, alert) in alerts.iter().enumerate() {
        let path = format!("$.a[{index}]");
        let Some(tuple) = alert.as_array() else {
            report.error(path, "alert must be a tuple array");
            continue;
        };
        if !(2..=3).contains(&tuple.len()) {
            report.error(path.clone(), "alert tuple must have 2 or 3 entries");
        }
        if let Some(level) = tuple.first().and_then(Value::as_str) {
            validate_string_length(
                level,
                &format!("{path}[0]"),
                limits.max_string_chars,
                "alert level code",
                report,
            );
            if !is_alert_level_code(level) {
                report.error(format!("{path}[0]"), "alert level code is unsupported");
            }
        } else {
            report.error(format!("{path}[0]"), "alert level code is unsupported");
        }
        for (item_index, item) in tuple.iter().enumerate().take(tuple.len().min(3)).skip(1) {
            if let Some(text) = item.as_str() {
                validate_string_length(
                    text,
                    &format!("{path}[{item_index}]"),
                    limits.max_string_chars,
                    "alert text",
                    report,
                );
            } else {
                report.error(
                    format!("{path}[{item_index}]"),
                    "alert text entries must be strings",
                );
            }
        }
    }
}

pub(super) fn validate_compact_markdown(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(sections) = value.as_array() else {
        report.error("$.md", "field 'md' must be an array when present");
        return;
    };
    validate_count(
        sections.len(),
        limits.max_markdown_sections,
        "$.md",
        "markdown sections",
        report,
    );
    let mut total_markdown_chars = 0usize;
    for (index, section) in sections.iter().enumerate() {
        let path = format!("$.md[{index}]");
        let Some(tuple) = section.as_array() else {
            report.error(path, "markdown section must be a tuple array");
            continue;
        };
        if !(1..=2).contains(&tuple.len()) {
            report.error(path.clone(), "markdown tuple must have 1 or 2 entries");
        }
        for (item_index, item) in tuple.iter().enumerate().take(tuple.len().min(2)) {
            let item_path = format!("{path}[{item_index}]");
            if let Some(text) = item.as_str() {
                let max_chars = if tuple.len() == 1 || item_index == 1 {
                    total_markdown_chars =
                        total_markdown_chars.saturating_add(text.chars().count());
                    limits.max_markdown_section_chars
                } else {
                    limits.max_string_chars
                };
                validate_string_length(text, &item_path, max_chars, "markdown entry", report);
            } else {
                report.error(item_path, "markdown entries must be strings");
            }
        }
    }
    if total_markdown_chars > limits.max_total_markdown_chars {
        report.error(
            "$.md",
            format!(
                "total markdown length {total_markdown_chars} chars exceeds max {}",
                limits.max_total_markdown_chars
            ),
        );
    }
}
