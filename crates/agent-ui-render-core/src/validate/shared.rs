use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::{diagnostic::ValidationReport, domain, options::Limits, wire::compact};

#[derive(Debug, Clone)]
pub(super) struct ColumnInfo {
    pub(super) key: String,
    pub(super) type_code: String,
}

#[derive(Debug, Clone)]
pub(super) struct DatasetInfo {
    pub(super) columns: Vec<ColumnInfo>,
    pub(super) rows: Vec<Vec<Value>>,
    pub(super) materialized: bool,
}

pub(super) fn validate_presentation_options(
    object: &Map<String, Value>,
    report: &mut ValidationReport,
) {
    validate_token(object, "theme", domain::THEMES, report);
    validate_token(object, "density", domain::DENSITIES, report);
    validate_token(object, "emphasis", domain::EMPHASES, report);
}

pub(super) fn validate_token(
    object: &Map<String, Value>,
    key: &str,
    allowed: &[&str],
    report: &mut ValidationReport,
) {
    if let Some(value) = object.get(key) {
        match value.as_str() {
            Some(text) if allowed.contains(&text) => {}
            Some(_) => report.error(
                format!("$.{key}"),
                format!("field '{key}' has unsupported value"),
            ),
            None => report.error(
                format!("$.{key}"),
                format!("field '{key}' must be a string when present"),
            ),
        }
    }
}

pub(super) fn validate_row_major(
    value: &Value,
    path: &str,
    column_count: usize,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Vec<Vec<Value>> {
    let Some(rows) = value.as_array() else {
        report.error(path, "row data must be an array of row arrays");
        return Vec::new();
    };
    validate_count(
        rows.len(),
        limits.max_rows_per_dataset,
        path,
        "rows",
        report,
    );
    let actual_cells = rows
        .iter()
        .filter_map(Value::as_array)
        .map(Vec::len)
        .sum::<usize>();
    validate_count(
        actual_cells,
        limits.max_cells_per_dataset,
        path,
        "cells",
        report,
    );
    let mut result = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let row_path = format!("{path}[{row_index}]");
        let Some(cells) = row.as_array() else {
            report.error(row_path, "dataset row must be an array");
            continue;
        };
        if cells.len() != column_count {
            report.error(
                row_path.clone(),
                format!(
                    "row length {} must equal column count {column_count}",
                    cells.len()
                ),
            );
        }
        for (cell_index, cell) in cells.iter().enumerate() {
            let cell_path = format!("{row_path}[{cell_index}]");
            if !is_primitive(cell) {
                report.error(
                    cell_path,
                    "row cells must be string, number, boolean, or null",
                );
            } else if let Some(text) = cell.as_str() {
                validate_string_length(
                    text,
                    &cell_path,
                    limits.max_string_chars,
                    "row cell",
                    report,
                );
            }
        }
        result.push(cells.clone());
    }
    result
}

pub(super) fn validate_string_array(
    name: &str,
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(items) = value.as_array() else {
        report.error(
            format!("$.{name}"),
            format!("field '{name}' must be an array when present"),
        );
        return;
    };
    for (index, item) in items.iter().enumerate() {
        if let Some(text) = item.as_str() {
            validate_string_length(
                text,
                &format!("$.{name}[{index}]"),
                limits.max_string_chars,
                &format!("field '{name}' entry"),
                report,
            );
        } else {
            report.error(
                format!("$.{name}[{index}]"),
                format!("field '{name}' entries must be strings"),
            );
        }
    }
}

pub(super) fn validate_dataset_id(dataset_id: &str, path: &str, report: &mut ValidationReport) {
    if dataset_id.is_empty() {
        report.error(path, "dataset id must not be empty");
    } else if !is_recommended_id(dataset_id) {
        report.warning(
            path.to_owned(),
            format!("dataset id '{dataset_id}' should be lower_snake_case"),
        );
    }
}

pub(super) fn validate_unknown_fields(
    path: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
    report: &mut ValidationReport,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            report.error(format!("{path}.{key}"), format!("unknown field '{key}'"));
        }
    }
}

pub(super) fn validate_count(
    actual: usize,
    max: usize,
    path: &str,
    label: &str,
    report: &mut ValidationReport,
) {
    if actual > max {
        report.error(path, format!("{label} count {actual} exceeds max {max}"));
    }
}

pub(super) fn validate_string_length(
    value: &str,
    path: &str,
    max: usize,
    label: &str,
    report: &mut ValidationReport,
) {
    let chars = value.chars().count();
    if chars > max {
        report.error(
            path,
            format!("{label} length {chars} chars exceeds max {max}"),
        );
    }
}

pub(super) fn is_type_code(value: &str, dictionaries: &BTreeMap<String, Vec<String>>) -> bool {
    if compact::is_base_or_dict_type_code(value) {
        if let Some(dict_id) = value.strip_prefix(compact::TYPE_CODE_DICT_PREFIX) {
            return dictionaries.contains_key(dict_id);
        }
        return true;
    }
    false
}

pub(super) fn is_base_or_dict_type_code(value: &str) -> bool {
    compact::is_base_or_dict_type_code(value)
}

pub(super) fn is_numeric_compact(column: &ColumnInfo) -> bool {
    matches!(
        column.type_code.as_str(),
        compact::TYPE_CODE_NUMBER | compact::TYPE_CODE_CURRENCY | compact::TYPE_CODE_PERCENT
    )
}

pub(super) fn is_numeric_normalized(column: &ColumnInfo) -> bool {
    matches!(
        column.type_code.as_str(),
        domain::COLUMN_TYPE_NUMBER | domain::COLUMN_TYPE_CURRENCY | domain::COLUMN_TYPE_PERCENT
    )
}

pub(super) fn is_normalized_column_type(value: &str) -> bool {
    domain::COLUMN_TYPES.contains(&value)
}

pub(super) fn is_alert_level_code(value: &str) -> bool {
    compact::is_alert_level_code(value)
}

pub(super) fn is_alert_level(value: &str) -> bool {
    domain::ALERT_LEVELS.contains(&value)
}

pub(super) fn normalized_view_requires_x(intent: &str) -> bool {
    matches!(
        intent,
        domain::VIEW_INTENT_TREND
            | domain::VIEW_INTENT_COMPARISON
            | domain::VIEW_INTENT_DISTRIBUTION
            | domain::VIEW_INTENT_COMPOSITION
            | domain::VIEW_INTENT_RELATIONSHIP
    )
}

pub(super) fn normalized_view_requires_measures(intent: &str) -> bool {
    matches!(
        intent,
        domain::VIEW_INTENT_TREND
            | domain::VIEW_INTENT_COMPARISON
            | domain::VIEW_INTENT_COMPOSITION
            | domain::VIEW_INTENT_RELATIONSHIP
    )
}

pub(super) fn distinct_category_count(dataset: &DatasetInfo, x_index: usize) -> usize {
    dataset
        .rows
        .iter()
        .map(|row| row.get(x_index).map_or("null".to_owned(), Value::to_string))
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn is_primitive(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
}

pub(super) fn is_recommended_id(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
