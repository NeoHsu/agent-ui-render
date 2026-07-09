mod unsafe_content;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::{
    diagnostic::ValidationReport,
    domain,
    options::{Limits, ValidationOptions},
    wire::compact,
};
use unsafe_content::collect_unsafe_string_paths;

const NORMALIZED_TOP_LEVEL_KEYS: &[&str] = &[
    "schema",
    "version",
    "title",
    "summary",
    "theme",
    "density",
    "emphasis",
    "datasets",
    "metrics",
    "insights",
    "markdown",
    "views",
    "alerts",
    "assumptions",
];

#[derive(Debug, Clone)]
struct ColumnInfo {
    key: String,
    type_code: String,
}

#[derive(Debug, Clone)]
struct DatasetInfo {
    columns: Vec<ColumnInfo>,
    rows: Vec<Vec<Value>>,
    materialized: bool,
}

#[must_use]
pub fn validate_report(value: &Value) -> ValidationReport {
    validate_report_with_options(value, &ValidationOptions::default())
}

#[must_use]
pub fn validate_report_with_options(
    value: &Value,
    options: &ValidationOptions,
) -> ValidationReport {
    validate_compact_report(value, &options.limits)
}

#[must_use]
fn validate_compact_report(value: &Value, limits: &Limits) -> ValidationReport {
    let mut report = ValidationReport::default();
    let Some(object) = value.as_object() else {
        report.error("$", "top-level value must be an object");
        return report;
    };

    validate_unknown_fields("$", object, compact::TOP_LEVEL_KEYS, &mut report);

    if object.get("version").and_then(Value::as_u64) != Some(compact::VERSION) {
        report.error("$.version", "field 'version' must be exactly 1");
    }

    for key in ["t", "s"] {
        if let Some(item) = object.get(key) {
            if let Some(text) = item.as_str() {
                validate_string_length(
                    text,
                    &format!("$.{key}"),
                    limits.max_string_chars,
                    &format!("field '{key}'"),
                    &mut report,
                );
            } else {
                report.error(
                    format!("$.{key}"),
                    format!("field '{key}' must be a string when present"),
                );
            }
        }
    }
    validate_presentation_options(object, &mut report);

    let dictionaries = validate_dictionaries(object.get("dict"), limits, &mut report);
    let datasets = validate_compact_datasets(object.get("d"), &dictionaries, limits, &mut report);

    validate_compact_metrics(object.get("m"), limits, &mut report);
    validate_compact_views(object.get("v"), &datasets, limits, &mut report);
    validate_compact_alerts(object.get("a"), limits, &mut report);
    validate_compact_markdown(object.get("md"), limits, &mut report);

    for path in collect_unsafe_string_paths(value) {
        report.error(
            path.clone(),
            format!("unsafe UI/code content detected at {path}"),
        );
    }

    let has_datasets = object
        .get("d")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let views_len = object
        .get("v")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let metrics_len = object
        .get("m")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let markdown_len = object
        .get("md")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if has_datasets
        && views_len == 0
        && metrics_len == 0
        && markdown_len == 0
        && object.get("s").is_none()
    {
        report.warning(
            "$",
            "compact payload has datasets but no views, metrics, markdown, or summary; renderer will likely fall back to a table",
        );
    }

    report
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
    let limits = &options.limits;
    let mut report = ValidationReport::default();
    let Some(object) = value.as_object() else {
        report.error("$", "top-level value must be an object");
        return report;
    };

    validate_unknown_fields("$", object, NORMALIZED_TOP_LEVEL_KEYS, &mut report);

    if object.get("schema").and_then(Value::as_str) != Some(domain::NORMALIZED_SCHEMA) {
        report.error(
            "$.schema",
            "field 'schema' must be exactly 'ui.input.normalized'",
        );
    }
    if object.get("version").and_then(Value::as_u64) != Some(u64::from(domain::FORMAT_VERSION)) {
        report.error("$.version", "field 'version' must be exactly 1");
    }

    for key in ["title", "summary"] {
        if let Some(item) = object.get(key) {
            if let Some(text) = item.as_str() {
                validate_string_length(
                    text,
                    &format!("$.{key}"),
                    limits.max_string_chars,
                    &format!("field '{key}'"),
                    &mut report,
                );
            } else {
                report.error(
                    format!("$.{key}"),
                    format!("field '{key}' must be a string when present"),
                );
            }
        }
    }
    validate_presentation_options(object, &mut report);

    let mut datasets_info: BTreeMap<String, DatasetInfo> = BTreeMap::new();
    let datasets = match object.get("datasets") {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => {
            report.error(
                "$.datasets",
                "field 'datasets' must be an object when present",
            );
            None
        }
    };
    if let Some(datasets) = datasets {
        validate_count(
            datasets.len(),
            limits.max_datasets,
            "$.datasets",
            "datasets",
            &mut report,
        );
        for (dataset_id, dataset) in datasets {
            let path = format!("$.datasets.{dataset_id}");
            if let Some(info) =
                validate_normalized_dataset(dataset_id, dataset, &path, limits, &mut report)
            {
                datasets_info.insert(dataset_id.clone(), info);
            }
        }
    }

    validate_normalized_metrics(object.get("metrics"), limits, &mut report);
    validate_string_array("insights", object.get("insights"), limits, &mut report);
    validate_normalized_markdown(object.get("markdown"), limits, &mut report);
    validate_string_array(
        "assumptions",
        object.get("assumptions"),
        limits,
        &mut report,
    );
    validate_normalized_views(object.get("views"), &datasets_info, limits, &mut report);
    validate_normalized_alerts(object.get("alerts"), limits, &mut report);

    for path in collect_unsafe_string_paths(value) {
        report.error(
            path.clone(),
            format!("unsafe UI/code content detected at {path}"),
        );
    }

    report
}

fn validate_presentation_options(object: &Map<String, Value>, report: &mut ValidationReport) {
    validate_token(object, "theme", domain::THEMES, report);
    validate_token(object, "density", domain::DENSITIES, report);
    validate_token(object, "emphasis", domain::EMPHASES, report);
}

fn validate_token(
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

fn validate_dictionaries(
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

fn validate_compact_datasets(
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

fn validate_row_major(
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

fn validate_compact_metrics(value: Option<&Value>, limits: &Limits, report: &mut ValidationReport) {
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
        if !(2..=4).contains(&tuple.len()) {
            report.error(path.clone(), "metric tuple must have 2 to 4 entries");
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
        if let Some(format) = tuple.get(2) {
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
        if let Some(unit) = tuple.get(3) {
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
    }
}

fn validate_compact_views(
    value: Option<&Value>,
    datasets: &[DatasetInfo],
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(views) = value.as_array() else {
        report.error("$.v", "field 'v' must be an array when present");
        return;
    };
    validate_count(views.len(), limits.max_views, "$.v", "views", report);
    for (index, view) in views.iter().enumerate() {
        validate_compact_view(view, index, datasets, limits, report);
    }
}

fn validate_compact_view(
    view: &Value,
    index: usize,
    datasets: &[DatasetInfo],
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let path = format!("$.v[{index}]");
    let Some(tuple) = view.as_array() else {
        report.error(path, "view must be a tuple array");
        return;
    };
    let Some(code) = tuple.first().and_then(Value::as_str) else {
        report.error(format!("{path}[0]"), "view code must be a string");
        return;
    };
    validate_string_length(
        code,
        &format!("{path}[0]"),
        limits.max_string_chars,
        "view code",
        report,
    );
    if !compact::is_view_code(code) {
        report.error(
            format!("{path}[0]"),
            format!("unsupported view code '{code}'"),
        );
        return;
    }
    let Some(data_index) = tuple
        .get(1)
        .and_then(Value::as_u64)
        .map(|item| item as usize)
    else {
        report.error(
            format!("{path}[1]"),
            "view dataset reference must be a non-negative integer",
        );
        return;
    };
    let Some(dataset) = datasets.get(data_index) else {
        report.error(
            format!("{path}[1]"),
            format!("view references missing dataset index {data_index}"),
        );
        return;
    };

    if code == compact::VIEW_CODE_OVERVIEW {
        if tuple.len() != 2 {
            report.error(path, "overview view tuple must have exactly 2 entries");
        }
        return;
    }
    if code == compact::VIEW_CODE_RECORDS {
        if !(2..=3).contains(&tuple.len()) {
            report.error(
                path.clone(),
                "records view tuple must be [\"r\", data] or [\"r\", data, columns]",
            );
            return;
        }
        if let Some(columns) = tuple.get(2) {
            if !dataset.materialized {
                report.error(
                    format!("{path}[2]"),
                    "records view column projection requires materialized dataset columns",
                );
                return;
            }
            validate_compact_record_columns(columns, &format!("{path}[2]"), dataset, report);
        }
        return;
    }
    if !dataset.materialized {
        return;
    }

    let x_index = tuple
        .get(2)
        .and_then(Value::as_u64)
        .map(|item| item as usize);
    let Some(x_index) = x_index else {
        report.error(
            format!("{path}[2]"),
            "view column index must be a non-negative integer",
        );
        return;
    };
    if x_index >= dataset.columns.len() {
        report.error(
            format!("{path}[2]"),
            format!("view column index {x_index} is out of range"),
        );
        return;
    }

    let measure_indexes = tuple.get(3).and_then(Value::as_array);
    if compact::is_measure_view_code(code) && measure_indexes.is_none_or(Vec::is_empty) {
        report.error(
            format!("{path}[3]"),
            "view requires at least one measure index",
        );
        return;
    }

    if code == compact::VIEW_CODE_RELATIONSHIP && !is_numeric_compact(&dataset.columns[x_index]) {
        report.error(
            format!("{path}[2]"),
            "relationship x column must be numeric-compatible",
        );
    }

    if let Some(measures) = measure_indexes {
        for (measure_pos, measure) in measures.iter().enumerate() {
            let measure_path = format!("{path}[3][{measure_pos}]");
            let Some(measure_index) = measure.as_u64().map(|item| item as usize) else {
                report.error(measure_path, "measure index must be a non-negative integer");
                continue;
            };
            if measure_index >= dataset.columns.len() {
                report.error(
                    measure_path,
                    format!("measure index {measure_index} is out of range"),
                );
                continue;
            }
            if !is_numeric_compact(&dataset.columns[measure_index]) {
                report.error(
                    measure_path.clone(),
                    "chart measure column must be numeric-compatible",
                );
            }
            if code == compact::VIEW_CODE_RELATIONSHIP && measure_index == x_index {
                report.error(
                    measure_path,
                    "relationship measure column must be distinct from x column",
                );
            }
        }
    }

    if code == compact::VIEW_CODE_COMPOSITION && distinct_category_count(dataset, x_index) > 5 {
        report.warning(
            path,
            format!(
                "view {index} composition has {} categories; renderer will fall back from pie to bar",
                distinct_category_count(dataset, x_index)
            ),
        );
    }
}

fn validate_compact_record_columns(
    value: &Value,
    path: &str,
    dataset: &DatasetInfo,
    report: &mut ValidationReport,
) {
    let Some(columns) = value.as_array() else {
        report.error(path, "records view columns must be an array");
        return;
    };
    if columns.is_empty() {
        report.error(path, "records view columns must not be empty");
    }

    let mut seen = BTreeSet::new();
    for (column_pos, column) in columns.iter().enumerate() {
        let column_path = format!("{path}[{column_pos}]");
        let Some(column_index) = column.as_u64().map(|item| item as usize) else {
            report.error(column_path, "column index must be a non-negative integer");
            continue;
        };
        if column_index >= dataset.columns.len() {
            report.error(
                column_path,
                format!("column index {column_index} is out of range"),
            );
            continue;
        }
        if !seen.insert(column_index) {
            report.error(
                column_path,
                format!("duplicate column index {column_index}"),
            );
        }
    }
}

fn validate_compact_alerts(value: Option<&Value>, limits: &Limits, report: &mut ValidationReport) {
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

fn validate_compact_markdown(
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

fn validate_normalized_dataset(
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

fn validate_normalized_metrics(
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

fn validate_string_array(
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

fn validate_normalized_markdown(
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

fn validate_normalized_views(
    value: Option<&Value>,
    datasets: &BTreeMap<String, DatasetInfo>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(views) = value.as_array() else {
        report.error("$.views", "field 'views' must be an array when present");
        return;
    };
    validate_count(views.len(), limits.max_views, "$.views", "views", report);
    for (index, view) in views.iter().enumerate() {
        validate_normalized_view(view, index, datasets, limits, report);
    }
}

fn validate_normalized_view(
    view: &Value,
    index: usize,
    datasets: &BTreeMap<String, DatasetInfo>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let path = format!("$.views[{index}]");
    let Some(object) = view.as_object() else {
        report.error(path, "view must be an object");
        return;
    };
    validate_unknown_fields(
        &path,
        object,
        &[
            "intent",
            "data",
            "x",
            "measures",
            "dimensions",
            "columns",
            "priority",
            "title",
        ],
        report,
    );
    let Some(intent) = object.get("intent").and_then(Value::as_str) else {
        report.error(format!("{path}.intent"), "view intent must be a string");
        return;
    };
    validate_string_length(
        intent,
        &format!("{path}.intent"),
        limits.max_string_chars,
        "view intent",
        report,
    );
    if !domain::VIEW_INTENTS.contains(&intent) {
        report.error(
            format!("{path}.intent"),
            format!("unsupported view intent '{intent}'"),
        );
        return;
    }
    let Some(data_id) = object.get("data").and_then(Value::as_str) else {
        report.error(format!("{path}.data"), "view data must be a string");
        return;
    };
    validate_string_length(
        data_id,
        &format!("{path}.data"),
        limits.max_string_chars,
        "view data",
        report,
    );
    let Some(dataset) = datasets.get(data_id) else {
        report.error(
            format!("{path}.data"),
            format!("view references missing dataset '{data_id}'"),
        );
        return;
    };
    validate_normalized_view_columns(object.get("columns"), &path, dataset, limits, report);
    if matches!(
        intent,
        domain::VIEW_INTENT_OVERVIEW | domain::VIEW_INTENT_PRECISE_RECORDS
    ) {
        return;
    }
    let x_key = object.get("x").and_then(Value::as_str);
    if normalized_view_requires_x(intent) && x_key.is_none() {
        report.error(format!("{path}.x"), "view requires x column key");
    }
    if let Some(x_key) = x_key {
        validate_string_length(
            x_key,
            &format!("{path}.x"),
            limits.max_string_chars,
            "view x column",
            report,
        );
        if !dataset.columns.iter().any(|column| column.key == x_key) {
            report.error(
                format!("{path}.x"),
                format!("view x column '{x_key}' does not exist"),
            );
        }
        if intent == domain::VIEW_INTENT_RELATIONSHIP
            && !dataset
                .columns
                .iter()
                .find(|column| column.key == x_key)
                .is_some_and(is_numeric_normalized)
        {
            report.error(
                format!("{path}.x"),
                "relationship x column must be numeric-compatible",
            );
        }
    }
    let measures = object.get("measures").and_then(Value::as_array);
    if normalized_view_requires_measures(intent) && measures.is_none_or(Vec::is_empty) {
        report.error(
            format!("{path}.measures"),
            "view requires at least one measure",
        );
        return;
    }
    if let Some(measures) = measures {
        for (measure_index, measure) in measures.iter().enumerate() {
            let measure_path = format!("{path}.measures[{measure_index}]");
            let Some(measure_key) = measure.as_str() else {
                report.error(measure_path, "measure key must be a string");
                continue;
            };
            validate_string_length(
                measure_key,
                &measure_path,
                limits.max_string_chars,
                "measure key",
                report,
            );
            let column = dataset
                .columns
                .iter()
                .find(|column| column.key == measure_key);
            let Some(column) = column else {
                report.error(
                    measure_path,
                    format!("measure column '{measure_key}' does not exist"),
                );
                continue;
            };
            if !is_numeric_normalized(column) {
                report.error(
                    measure_path.clone(),
                    "chart measure column must be numeric-compatible",
                );
            }
            if intent == domain::VIEW_INTENT_RELATIONSHIP && Some(measure_key) == x_key {
                report.error(
                    measure_path,
                    "relationship measure column must be distinct from x column",
                );
            }
        }
    }
}

fn validate_normalized_view_columns(
    value: Option<&Value>,
    path: &str,
    dataset: &DatasetInfo,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    let Some(value) = value else {
        return;
    };
    let Some(columns) = value.as_array() else {
        report.error(
            format!("{path}.columns"),
            "view columns must be an array when present",
        );
        return;
    };
    if columns.is_empty() {
        report.error(format!("{path}.columns"), "view columns must not be empty");
    }

    let mut seen = BTreeSet::new();
    for (column_index, column) in columns.iter().enumerate() {
        let column_path = format!("{path}.columns[{column_index}]");
        let Some(column_key) = column.as_str() else {
            report.error(column_path, "column key must be a string");
            continue;
        };
        validate_string_length(
            column_key,
            &column_path,
            limits.max_string_chars,
            "column key",
            report,
        );
        if !dataset
            .columns
            .iter()
            .any(|column| column.key == column_key)
        {
            report.error(
                column_path.clone(),
                format!("column '{column_key}' does not exist"),
            );
        }
        if !seen.insert(column_key) {
            report.error(column_path, format!("duplicate column '{column_key}'"));
        }
    }
}

fn validate_normalized_alerts(
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

fn validate_dataset_id(dataset_id: &str, path: &str, report: &mut ValidationReport) {
    if dataset_id.is_empty() {
        report.error(path, "dataset id must not be empty");
    } else if !is_recommended_id(dataset_id) {
        report.warning(
            path.to_owned(),
            format!("dataset id '{dataset_id}' should be lower_snake_case"),
        );
    }
}

fn validate_unknown_fields(
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

fn validate_count(
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

fn validate_string_length(
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

fn is_type_code(value: &str, dictionaries: &BTreeMap<String, Vec<String>>) -> bool {
    if compact::is_base_or_dict_type_code(value) {
        if let Some(dict_id) = value.strip_prefix(compact::TYPE_CODE_DICT_PREFIX) {
            return dictionaries.contains_key(dict_id);
        }
        return true;
    }
    false
}

fn is_base_or_dict_type_code(value: &str) -> bool {
    compact::is_base_or_dict_type_code(value)
}

fn is_numeric_compact(column: &ColumnInfo) -> bool {
    matches!(
        column.type_code.as_str(),
        compact::TYPE_CODE_NUMBER | compact::TYPE_CODE_CURRENCY | compact::TYPE_CODE_PERCENT
    )
}

fn is_numeric_normalized(column: &ColumnInfo) -> bool {
    matches!(
        column.type_code.as_str(),
        domain::COLUMN_TYPE_NUMBER | domain::COLUMN_TYPE_CURRENCY | domain::COLUMN_TYPE_PERCENT
    )
}

fn is_normalized_column_type(value: &str) -> bool {
    domain::COLUMN_TYPES.contains(&value)
}

fn is_alert_level_code(value: &str) -> bool {
    compact::is_alert_level_code(value)
}

fn is_alert_level(value: &str) -> bool {
    domain::ALERT_LEVELS.contains(&value)
}

fn normalized_view_requires_x(intent: &str) -> bool {
    matches!(
        intent,
        domain::VIEW_INTENT_TREND
            | domain::VIEW_INTENT_COMPARISON
            | domain::VIEW_INTENT_DISTRIBUTION
            | domain::VIEW_INTENT_COMPOSITION
            | domain::VIEW_INTENT_RELATIONSHIP
    )
}

fn normalized_view_requires_measures(intent: &str) -> bool {
    matches!(
        intent,
        domain::VIEW_INTENT_TREND
            | domain::VIEW_INTENT_COMPARISON
            | domain::VIEW_INTENT_COMPOSITION
            | domain::VIEW_INTENT_RELATIONSHIP
    )
}

fn distinct_category_count(dataset: &DatasetInfo, x_index: usize) -> usize {
    dataset
        .rows
        .iter()
        .map(|row| row.get(x_index).map_or("null".to_owned(), Value::to_string))
        .collect::<BTreeSet<_>>()
        .len()
}

fn is_primitive(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
}

fn is_recommended_id(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
