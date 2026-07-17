use std::collections::BTreeSet;

use serde_json::Value;

use crate::{diagnostic::ValidationReport, options::Limits, wire::compact};

use super::super::shared::{
    DatasetInfo, distinct_category_count, is_numeric_compact, validate_count,
    validate_string_length,
};

pub(super) fn validate_compact_views(
    value: Option<&Value>,
    version: u64,
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
        let code = view
            .as_array()
            .and_then(|tuple| tuple.first())
            .and_then(Value::as_str);
        if version == crate::wire::v2::VERSION
            && code.is_some_and(|item| !compact::is_view_code(item))
        {
            validate_compact_v2_view(view, index, datasets, report);
        } else {
            validate_compact_view(view, index, datasets, limits, report);
        }
    }
}

fn validate_compact_v2_view(
    view: &Value,
    index: usize,
    datasets: &[DatasetInfo],
    report: &mut ValidationReport,
) {
    let metas = datasets
        .iter()
        .enumerate()
        .map(|(dataset_index, dataset)| crate::wire::v2::DatasetMeta {
            id: format!("dataset_{dataset_index}"),
            columns: dataset
                .columns
                .iter()
                .map(|column| crate::wire::v2::ColumnMeta {
                    key: column.key.clone(),
                    type_code: column.type_code.clone(),
                })
                .collect(),
            materialized: dataset.materialized,
        })
        .collect::<Vec<_>>();
    if let Err(message) = crate::wire::v2::normalize_view(view, index, &metas) {
        report.error(format!("$.v[{index}]"), message);
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
