use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::{diagnostic::ValidationReport, domain, options::Limits};

use super::super::shared::{
    DatasetInfo, is_numeric_normalized, normalized_view_requires_measures,
    normalized_view_requires_x, validate_count, validate_string_length, validate_unknown_fields,
};

pub(super) fn validate_normalized_views(
    value: Option<&Value>,
    version: u64,
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
        validate_normalized_view(view, index, version, datasets, limits, report);
    }
}

fn validate_normalized_view(
    view: &Value,
    index: usize,
    version: u64,
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
            "chart",
            "datasets",
            "spec",
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
    if intent == domain::VIEW_INTENT_CHART {
        validate_normalized_chart_view(object, &path, version, datasets, limits, report);
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

fn validate_normalized_chart_view(
    object: &Map<String, Value>,
    path: &str,
    version: u64,
    datasets: &BTreeMap<String, DatasetInfo>,
    limits: &Limits,
    report: &mut ValidationReport,
) {
    if version != u64::from(domain::FORMAT_VERSION_V2) {
        report.error(
            format!("{path}.intent"),
            "explicit chart views require normalized input version 2",
        );
    }
    let Some(data_id) = object.get("data").and_then(Value::as_str) else {
        report.error(format!("{path}.data"), "chart data must be a string");
        return;
    };
    validate_string_length(
        data_id,
        &format!("{path}.data"),
        limits.max_string_chars,
        "chart data",
        report,
    );
    if !datasets.contains_key(data_id) {
        report.error(
            format!("{path}.data"),
            format!("chart references missing dataset '{data_id}'"),
        );
    }
    let Some(chart) = object.get("chart").and_then(Value::as_str) else {
        report.error(format!("{path}.chart"), "chart type must be a string");
        return;
    };
    if !crate::wire::v2::NORMALIZED_CHARTS.contains(&chart) {
        report.error(
            format!("{path}.chart"),
            format!("unsupported normalized chart type '{chart}'"),
        );
    }
    let Some(dataset_refs) = object.get("datasets").and_then(Value::as_array) else {
        report.error(
            format!("{path}.datasets"),
            "chart datasets must be a non-empty array",
        );
        return;
    };
    if dataset_refs.is_empty() {
        report.error(
            format!("{path}.datasets"),
            "chart datasets must not be empty",
        );
    }
    for (index, dataset_ref) in dataset_refs.iter().enumerate() {
        let reference_path = format!("{path}.datasets[{index}]");
        let Some(reference) = dataset_ref.as_str() else {
            report.error(reference_path, "chart dataset reference must be a string");
            continue;
        };
        if !datasets.contains_key(reference) {
            report.error(
                reference_path,
                format!("chart references missing dataset '{reference}'"),
            );
        }
    }
    let Some(spec) = object.get("spec").and_then(Value::as_object) else {
        report.error(format!("{path}.spec"), "chart spec must be an object");
        return;
    };
    if spec.get("$schema").and_then(Value::as_str) != Some(crate::wire::v2::VEGA_LITE_SCHEMA) {
        report.error(
            format!("{path}.spec.$schema"),
            "chart spec must use the bundled Vega-Lite schema version",
        );
    }
    if contains_prohibited_vega_key(&Value::Object(spec.clone())) {
        report.error(
            format!("{path}.spec"),
            "chart spec contains a prohibited URL, href, image, or geoshape capability",
        );
    }
}

fn contains_prohibited_vega_key(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            matches!(key.as_str(), "url" | "href")
                || (key == "mark"
                    && match value {
                        Value::String(mark) => matches!(mark.as_str(), "image" | "geoshape"),
                        Value::Object(mark) => mark
                            .get("type")
                            .and_then(Value::as_str)
                            .is_some_and(|mark| matches!(mark, "image" | "geoshape")),
                        _ => false,
                    })
                || contains_prohibited_vega_key(value)
        }),
        Value::Array(items) => items.iter().any(contains_prohibited_vega_key),
        _ => false,
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
