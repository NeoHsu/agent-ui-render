use std::collections::BTreeSet;

use serde_json::Value;

use crate::{diagnostic::ValidationReport, domain, options::Limits};

use super::super::shared::{
    ColumnInfo, DatasetInfo, is_normalized_column_type, is_recommended_id, validate_count,
    validate_dataset_id, validate_row_major, validate_string_length, validate_unknown_fields,
};

pub(super) fn validate_normalized_dataset(
    dataset_id: &str,
    value: &Value,
    path: &str,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Option<DatasetInfo> {
    validate_string_length(
        dataset_id,
        path,
        limits.max_string_chars,
        "dataset id",
        report,
    );
    validate_dataset_id(dataset_id, path, report);
    let Some(object) = value.as_object() else {
        report.error(path, "dataset must be an object");
        return None;
    };
    validate_unknown_fields(path, object, &["columns", "rows"], report);
    let Some(columns_value) = object.get("columns") else {
        report.error(format!("{path}.columns"), "dataset must include columns");
        return None;
    };
    let Some(columns_array) = columns_value.as_array() else {
        report.error(
            format!("{path}.columns"),
            "dataset columns must be an array",
        );
        return None;
    };
    if columns_array.is_empty() {
        report.error(
            format!("{path}.columns"),
            "dataset columns must not be empty",
        );
    }
    validate_count(
        columns_array.len(),
        limits.max_columns_per_dataset,
        &format!("{path}.columns"),
        "columns",
        report,
    );
    let mut columns = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, column) in columns_array.iter().enumerate() {
        if let Some(info) =
            validate_normalized_column(column, &format!("{path}.columns[{index}]"), limits, report)
        {
            if !seen.insert(info.key.clone()) {
                report.error(
                    format!("{path}.columns[{index}].key"),
                    format!("duplicate column key '{}'", info.key),
                );
            }
            columns.push(info);
        }
    }
    let rows = if let Some(rows_value) = object.get("rows") {
        validate_row_major(
            rows_value,
            &format!("{path}.rows"),
            columns.len(),
            limits,
            report,
        )
    } else {
        report.error(format!("{path}.rows"), "dataset must include rows");
        Vec::new()
    };
    Some(DatasetInfo {
        columns,
        rows,
        materialized: true,
    })
}

fn validate_normalized_column(
    value: &Value,
    path: &str,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Option<ColumnInfo> {
    let Some(object) = value.as_object() else {
        report.error(path, "column must be an object");
        return None;
    };
    validate_unknown_fields(
        path,
        object,
        &["key", "label", "type", "unit", "description"],
        report,
    );
    let Some(key) = object.get("key").and_then(Value::as_str) else {
        report.error(format!("{path}.key"), "column key must be a string");
        return None;
    };
    validate_string_length(
        key,
        &format!("{path}.key"),
        limits.max_string_chars,
        "column key",
        report,
    );
    if !is_recommended_id(key) {
        report.warning(
            format!("{path}.key"),
            format!("column key '{key}' should be lower_snake_case"),
        );
    }
    let column_type = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or(domain::COLUMN_TYPE_STRING);
    validate_string_length(
        column_type,
        &format!("{path}.type"),
        limits.max_string_chars,
        "column type",
        report,
    );
    if !is_normalized_column_type(column_type) {
        report.error(
            format!("{path}.type"),
            format!("unsupported column type '{column_type}'"),
        );
    }
    for key in ["label", "unit", "description"] {
        if let Some(item) = object.get(key) {
            if let Some(text) = item.as_str() {
                validate_string_length(
                    text,
                    &format!("{path}.{key}"),
                    limits.max_string_chars,
                    &format!("column {key}"),
                    report,
                );
            } else {
                report.error(
                    format!("{path}.{key}"),
                    format!("column {key} must be a string when present"),
                );
            }
        }
    }
    Some(ColumnInfo {
        key: key.to_owned(),
        type_code: column_type.to_owned(),
    })
}
