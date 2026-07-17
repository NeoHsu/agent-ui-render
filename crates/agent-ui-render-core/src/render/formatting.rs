use serde_json::Value;

use crate::domain::{Column, Dataset, Metric, Primitive, Report, ViewIntent};

pub(super) fn format_metric(metric: &Metric) -> String {
    match metric.format.as_deref() {
        Some("currency") => format_currency(metric.value.as_f64(), metric.unit.as_deref()),
        Some("percent") => metric.value.as_f64().map_or_else(
            || cell_plain(Some(&metric.value)),
            |value| format!("{:.1}%", value * 100.0),
        ),
        Some("number") => metric
            .value
            .as_f64()
            .map_or_else(|| cell_plain(Some(&metric.value)), format_number),
        _ => cell_plain(Some(&metric.value)),
    }
}

pub(super) fn format_cell_value(value: &Primitive, column: Option<&Column>) -> String {
    match column.and_then(|column| column.column_type.as_deref()) {
        Some("currency") => format_currency(
            value.as_f64(),
            column.and_then(|column| column.unit.as_deref()),
        ),
        Some("percent") => value.as_f64().map_or_else(
            || cell_plain(Some(value)),
            |number| format!("{:.1}%", number * 100.0),
        ),
        Some("number") => value
            .as_f64()
            .map_or_else(|| cell_plain(Some(value)), format_number),
        _ => cell_plain(Some(value)),
    }
}

pub(super) fn format_currency(value: Option<f64>, unit: Option<&str>) -> String {
    value.map_or_else(
        || "—".to_owned(),
        |number| match unit {
            Some(unit) if !unit.is_empty() => format!("{unit} {}", format_number(number)),
            _ => format_number(number),
        },
    )
}

pub(super) fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

pub(super) fn cell_plain(value: Option<&Value>) -> String {
    match value {
        Some(Value::Null) | None => "—".to_owned(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(other) => other.to_string(),
    }
}

pub(super) fn column_label(dataset: &Dataset, key: &str) -> String {
    dataset
        .columns
        .iter()
        .find(|column| column.key == key)
        .and_then(|column| column.label.clone())
        .unwrap_or_else(|| key.to_owned())
}

pub(super) fn chart_aria_label(view: &ViewIntent) -> String {
    format!("{} chart for {}", view.intent.replace('_', " "), view.data)
}

pub(super) fn view_title(view: &ViewIntent, index: usize) -> String {
    match view.intent.as_str() {
        "precise_records" => "Records".to_owned(),
        "trend" => "Trend".to_owned(),
        "comparison" => "Comparison".to_owned(),
        "distribution" => "Distribution".to_owned(),
        "composition" => "Composition".to_owned(),
        "relationship" => "Relationship".to_owned(),
        "overview" => "Overview".to_owned(),
        _ => format!("View {}", index + 1),
    }
}

pub(super) fn titleize_chart_name(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                format!("{}{}", first.to_uppercase(), chars.as_str())
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn script_safe_json(input: &Report) -> String {
    serde_json::to_string(input)
        .unwrap_or_else(|_| "{}".to_owned())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

pub(super) fn class_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
