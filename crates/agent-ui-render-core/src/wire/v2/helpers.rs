use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::{ColumnMeta, DatasetMeta};

pub(super) fn field_def(
    dataset: &DatasetMeta,
    index: usize,
    quantitative: bool,
) -> Result<Value, String> {
    let column = column(dataset, index)?;
    Ok(json!({
        "field": column.key,
        "type": if quantitative { "quantitative" } else { vega_type(&column.type_code) },
        "title": titleize(&column.key)
    }))
}

fn vega_type(type_code: &str) -> &'static str {
    match type_code {
        "n" | "cur" | "pct" => "quantitative",
        "d" | "dt" => "temporal",
        _ => "nominal",
    }
}

pub(super) fn chart_name(code: &str) -> &'static str {
    match code {
        "ln" => "line",
        "ar" => "area",
        "tr" => "trail",
        "reg" => "regression",
        "bar" => "bar",
        "gantt" => "gantt",
        "water" => "waterfall",
        "bullet" => "bullet",
        "range" => "ranged-dot",
        "hist" => "histogram",
        "den" => "density",
        "dot" => "dot",
        "box" | "box5" => "boxplot",
        "qq" => "qq",
        "sc" => "scatter",
        "par" => "parallel-coordinates",
        "tri" => "ternary",
        "heat" => "heatmap",
        "mos" => "mosaic",
        "pie" => "pie",
        "don" => "donut",
        "rad" => "radial",
        "err" => "errorbar",
        "band" => "errorband",
        "candle" => "candlestick",
        "text" => "text",
        "tick" => "tick",
        "rule" => "rule",
        _ => "chart",
    }
}

pub(super) fn column(dataset: &DatasetMeta, index: usize) -> Result<&ColumnMeta, String> {
    dataset.columns.get(index).ok_or_else(|| {
        format!(
            "column index {index} is out of range for dataset '{}'",
            dataset.id
        )
    })
}

pub(super) fn ensure_numeric(
    dataset: &DatasetMeta,
    indexes: &[usize],
    role: &str,
) -> Result<(), String> {
    for index in indexes {
        let column = column(dataset, *index)?;
        if !matches!(column.type_code.as_str(), "n" | "cur" | "pct") {
            return Err(format!(
                "{role} column '{}' must be numeric-compatible",
                column.key
            ));
        }
    }
    Ok(())
}

pub(super) fn series_legend_label_expr(fields: &[String]) -> String {
    fields
        .iter()
        .rev()
        .fold("datum.label".to_owned(), |fallback, field| {
            let raw = serde_json::to_string(field).unwrap_or_else(|_| "\"\"".to_owned());
            let label = serde_json::to_string(&titleize(field)).unwrap_or_else(|_| raw.clone());
            format!("datum.label === {raw} ? {label} : ({fallback})")
        })
}

pub(super) fn datum_field(field: &str) -> String {
    format!(
        "datum[{}]",
        serde_json::to_string(field).unwrap_or_else(|_| "\"\"".to_owned())
    )
}

pub(super) fn titleize(value: &str) -> String {
    value
        .split('_')
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

pub(super) fn remove_schema(spec: &mut Value) {
    if let Some(object) = spec.as_object_mut() {
        object.remove("$schema");
    }
}

pub(super) fn remove_data(spec: &mut Value) {
    if let Some(object) = spec.as_object_mut() {
        object.remove("data");
    }
}

pub(super) fn deduplicate(values: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    values.retain(|item| seen.insert(item.clone()));
}

pub(super) fn replace_first_quantitative_field(value: &mut Value, replacement: &Value) -> bool {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("quantitative")
                && object.contains_key("field")
            {
                object.insert("field".to_owned(), replacement.clone());
                return true;
            }
            object
                .values_mut()
                .any(|item| replace_first_quantitative_field(item, replacement))
        }
        Value::Array(items) => items
            .iter_mut()
            .any(|item| replace_first_quantitative_field(item, replacement)),
        _ => false,
    }
}
