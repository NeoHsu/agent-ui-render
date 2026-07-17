use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{diagnostic::ValidationReport, options::Limits, wire::compact};

use super::super::shared::{
    ColumnInfo, DatasetInfo, is_primitive, is_recommended_id, is_type_code, validate_count,
    validate_dataset_id, validate_row_major, validate_string_length,
};

pub(super) fn validate_dictionaries(
    value: Option<&Value>,
    limits: &Limits,
    report: &mut ValidationReport,
) -> BTreeMap<String, Vec<String>> {
    let mut dictionaries = BTreeMap::new();
    let Some(value) = value else {
        return dictionaries;
    };
    let Some(object) = value.as_object() else {
        report.error("$.dict", "field 'dict' must be an object when present");
        return dictionaries;
    };
    for (dict_id, entries) in object {
        validate_string_length(
            dict_id,
            &format!("$.dict.{dict_id}"),
            limits.max_string_chars,
            "dictionary id",
            report,
        );
        if !is_recommended_id(dict_id) {
            report.warning(
                format!("$.dict.{dict_id}"),
                format!("dictionary id '{dict_id}' should be lower_snake_case"),
            );
        }
        let Some(array) = entries.as_array() else {
            report.error(
                format!("$.dict.{dict_id}"),
                "dictionary entries must be an array of strings",
            );
            continue;
        };
        let mut strings = Vec::new();
        for (index, item) in array.iter().enumerate() {
            if let Some(text) = item.as_str() {
                validate_string_length(
                    text,
                    &format!("$.dict.{dict_id}[{index}]"),
                    limits.max_string_chars,
                    "dictionary entry",
                    report,
                );
                strings.push(text.to_owned());
            } else {
                report.error(
                    format!("$.dict.{dict_id}[{index}]"),
                    "dictionary entries must be strings",
                );
            }
        }
        dictionaries.insert(dict_id.clone(), strings);
    }
    dictionaries
}

pub(super) fn validate_compact_datasets(
    value: Option<&Value>,
    dictionaries: &BTreeMap<String, Vec<String>>,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Vec<DatasetInfo> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(datasets) = value.as_array() else {
        report.error("$.d", "field 'd' must be an array when present");
        return Vec::new();
    };
    validate_count(
        datasets.len(),
        limits.max_datasets,
        "$.d",
        "datasets",
        report,
    );
    let mut seen_ids = BTreeSet::new();
    datasets
        .iter()
        .enumerate()
        .filter_map(|(index, dataset)| {
            validate_compact_dataset(
                dataset,
                &format!("$.d[{index}]"),
                dictionaries,
                limits,
                &mut seen_ids,
                report,
            )
        })
        .collect()
}

fn validate_compact_dataset(
    value: &Value,
    path: &str,
    dictionaries: &BTreeMap<String, Vec<String>>,
    limits: &Limits,
    seen_ids: &mut BTreeSet<String>,
    report: &mut ValidationReport,
) -> Option<DatasetInfo> {
    let Some(tuple) = value.as_array() else {
        report.error(path, "dataset must be a tuple array");
        return None;
    };
    if tuple.len() < 3 || tuple.len() > 4 {
        report.error(
            path,
            "dataset tuple must be [id, columns, rows], [id, 'cols', columns, columnData], or [id, 'ref', ref]",
        );
        return None;
    }
    let Some(dataset_id) = tuple.first().and_then(Value::as_str) else {
        report.error(format!("{path}[0]"), "dataset id must be a string");
        return None;
    };
    validate_string_length(
        dataset_id,
        &format!("{path}[0]"),
        limits.max_string_chars,
        "dataset id",
        report,
    );
    validate_dataset_id(dataset_id, path, report);
    if !seen_ids.insert(dataset_id.to_owned()) {
        report.error(
            format!("{path}[0]"),
            format!("duplicate dataset id '{dataset_id}'"),
        );
    }

    match tuple.get(1).and_then(Value::as_str) {
        Some("ref") => {
            if tuple.len() != 3 {
                report.error(path, "ref dataset tuple must have exactly 3 entries");
            }
            if let Some(reference) = tuple.get(2).and_then(Value::as_str) {
                validate_string_length(
                    reference,
                    &format!("{path}[2]"),
                    limits.max_string_chars,
                    "dataset ref",
                    report,
                );
            } else {
                report.error(format!("{path}[2]"), "dataset ref must be a string");
            }
            Some(DatasetInfo {
                columns: Vec::new(),
                rows: Vec::new(),
                materialized: false,
            })
        }
        Some("cols") => {
            if tuple.len() != 4 {
                report.error(
                    path,
                    "column-major dataset tuple must have exactly 4 entries",
                );
            }
            let columns = validate_compact_columns(
                tuple.get(2),
                &format!("{path}[2]"),
                dictionaries,
                limits,
                report,
            );
            let rows = validate_column_major(
                tuple.get(3).unwrap_or(&Value::Null),
                &format!("{path}[3]"),
                columns.len(),
                limits,
                report,
            );
            Some(DatasetInfo {
                columns,
                rows,
                materialized: true,
            })
        }
        Some(other) => {
            report.error(
                format!("{path}[1]"),
                format!("unsupported dataset mode '{other}'"),
            );
            None
        }
        None => {
            if tuple.len() != 3 {
                report.error(path, "row-major dataset tuple must have exactly 3 entries");
            }
            let columns = validate_compact_columns(
                tuple.get(1),
                &format!("{path}[1]"),
                dictionaries,
                limits,
                report,
            );
            let rows = validate_row_major(
                tuple.get(2).unwrap_or(&Value::Null),
                &format!("{path}[2]"),
                columns.len(),
                limits,
                report,
            );
            Some(DatasetInfo {
                columns,
                rows,
                materialized: true,
            })
        }
    }
}

fn validate_compact_columns(
    value: Option<&Value>,
    path: &str,
    dictionaries: &BTreeMap<String, Vec<String>>,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Vec<ColumnInfo> {
    let Some(columns_value) = value else {
        report.error(path, "dataset must include column tuples");
        return Vec::new();
    };
    let Some(columns_array) = columns_value.as_array() else {
        report.error(path, "dataset columns must be an array of column tuples");
        return Vec::new();
    };
    if columns_array.is_empty() {
        report.error(path, "dataset columns must not be empty");
    }
    validate_count(
        columns_array.len(),
        limits.max_columns_per_dataset,
        path,
        "columns",
        report,
    );
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    for (index, column) in columns_array.iter().enumerate() {
        if let Some(info) = validate_compact_column(
            column,
            &format!("{path}[{index}]"),
            dictionaries,
            limits,
            report,
        ) {
            if !seen.insert(info.key.clone()) {
                report.error(
                    format!("{path}[{index}][0]"),
                    format!("duplicate column key '{}'", info.key),
                );
            }
            columns.push(info);
        }
    }
    columns
}

fn validate_compact_column(
    value: &Value,
    path: &str,
    dictionaries: &BTreeMap<String, Vec<String>>,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Option<ColumnInfo> {
    let Some(tuple) = value.as_array() else {
        report.error(path, "column must be a tuple array");
        return None;
    };
    if !(2..=4).contains(&tuple.len()) {
        report.error(path, "column tuple must have 2 to 4 entries");
        return None;
    }
    let Some(key) = tuple.first().and_then(Value::as_str) else {
        report.error(format!("{path}[0]"), "column key must be a string");
        return None;
    };
    validate_string_length(
        key,
        &format!("{path}[0]"),
        limits.max_string_chars,
        "column key",
        report,
    );
    if key.is_empty() {
        report.error(format!("{path}[0]"), "column key must not be empty");
    } else if !is_recommended_id(key) {
        report.warning(
            format!("{path}[0]"),
            format!("column key '{key}' should be lower_snake_case"),
        );
    }
    let Some(type_code) = tuple.get(1).and_then(Value::as_str) else {
        report.error(format!("{path}[1]"), "column type code must be a string");
        return Some(ColumnInfo {
            key: key.to_owned(),
            type_code: compact::TYPE_CODE_STRING.to_owned(),
        });
    };
    validate_string_length(
        type_code,
        &format!("{path}[1]"),
        limits.max_string_chars,
        "column type code",
        report,
    );
    if !is_type_code(type_code, dictionaries) {
        report.error(
            format!("{path}[1]"),
            format!("unsupported column type code '{type_code}'"),
        );
    }
    if let Some(unit) = tuple.get(2) {
        if let Some(unit) = unit.as_str() {
            validate_string_length(
                unit,
                &format!("{path}[2]"),
                limits.max_string_chars,
                "column unit",
                report,
            );
        } else {
            report.error(
                format!("{path}[2]"),
                "column unit must be a string when present",
            );
        }
    }
    if let Some(label) = tuple.get(3) {
        if let Some(label) = label.as_str() {
            validate_string_length(
                label,
                &format!("{path}[3]"),
                limits.max_string_chars,
                "column label",
                report,
            );
        } else {
            report.error(
                format!("{path}[3]"),
                "column label must be a string when present",
            );
        }
    }
    Some(ColumnInfo {
        key: key.to_owned(),
        type_code: type_code.to_owned(),
    })
}

fn validate_column_major(
    value: &Value,
    path: &str,
    column_count: usize,
    limits: &Limits,
    report: &mut ValidationReport,
) -> Vec<Vec<Value>> {
    let Some(columns) = value.as_array() else {
        report.error(path, "column-major data must be an array of column arrays");
        return Vec::new();
    };
    if columns.len() != column_count {
        report.error(
            path,
            format!(
                "column-major data has {} columns but schema has {column_count}",
                columns.len()
            ),
        );
    }
    let actual_cells = columns
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
    let mut column_values = Vec::new();
    let mut row_count: Option<usize> = None;
    for (column_index, column) in columns.iter().enumerate() {
        let column_path = format!("{path}[{column_index}]");
        let Some(cells) = column.as_array() else {
            report.error(column_path, "column-major entry must be an array");
            continue;
        };
        if let Some(expected) = row_count {
            if cells.len() != expected {
                report.error(
                    column_path.clone(),
                    format!(
                        "column length {} must equal first column length {expected}",
                        cells.len()
                    ),
                );
            }
        } else {
            row_count = Some(cells.len());
            validate_count(
                cells.len(),
                limits.max_rows_per_dataset,
                path,
                "rows",
                report,
            );
        }
        for (cell_index, cell) in cells.iter().enumerate() {
            let cell_path = format!("{column_path}[{cell_index}]");
            if !is_primitive(cell) {
                report.error(
                    cell_path,
                    "column-major cells must be string, number, boolean, or null",
                );
            } else if let Some(text) = cell.as_str() {
                validate_string_length(
                    text,
                    &cell_path,
                    limits.max_string_chars,
                    "column-major cell",
                    report,
                );
            }
        }
        column_values.push(cells.clone());
    }

    let rows = row_count.unwrap_or(0);
    (0..rows)
        .map(|row_index| {
            column_values
                .iter()
                .map(|column| column.get(row_index).cloned().unwrap_or(Value::Null))
                .collect()
        })
        .collect()
}
