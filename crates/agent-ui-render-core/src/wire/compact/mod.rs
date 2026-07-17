mod codes;
mod content;
mod datasets;
mod views;

use serde_json::Value;

use crate::{
    diagnostic::{Finding, FindingLevel},
    domain::{self, Alert, Column, Dataset, Report},
};

use self::{
    content::{
        normalize_compact_alerts, normalize_compact_markdown, normalize_compact_metrics,
        normalize_compact_string_list,
    },
    datasets::{normalize_compact_dataset, read_dictionaries},
    views::normalize_compact_views,
};

pub use codes::{
    ALERT_CODE_CRITICAL, ALERT_CODE_ERROR, ALERT_CODE_INFO, ALERT_CODE_SUCCESS, ALERT_CODE_WARNING,
    ALERT_LEVEL_CODES, BASE_TYPE_CODES, DELTA_FORMAT_CODES, MEASURE_VIEW_CODES, SIMPLE_VIEW_CODES,
    TOP_LEVEL_KEYS, TYPE_CODE_BOOLEAN, TYPE_CODE_CURRENCY, TYPE_CODE_DATE, TYPE_CODE_DATETIME,
    TYPE_CODE_DICT_PREFIX, TYPE_CODE_NUMBER, TYPE_CODE_PERCENT, TYPE_CODE_STRING, VERSION,
    VIEW_CODE_COMPARISON, VIEW_CODE_COMPOSITION, VIEW_CODE_DISTRIBUTION, VIEW_CODE_OVERVIEW,
    VIEW_CODE_RECORDS, VIEW_CODE_RELATIONSHIP, VIEW_CODE_TREND, VIEW_CODES, is_alert_level_code,
    is_base_or_dict_type_code, is_measure_view_code, is_simple_view_code, is_view_code,
    normalize_alert_level, normalize_metric_format, normalize_type_code, view_intent_for_code,
};

#[derive(Debug, Clone)]
pub(crate) struct CompactColumnMeta {
    pub(crate) key: String,
    pub(crate) type_code: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CompactDatasetMeta {
    pub(crate) id: String,
    pub(crate) columns: Vec<CompactColumnMeta>,
    pub(crate) materialized: bool,
}

#[must_use]
pub fn normalize_compact_report(value: &Value) -> (Report, Vec<Finding>) {
    let mut input = Report::default();
    let Some(object) = value.as_object() else {
        return (
            input,
            vec![Finding {
                level: FindingLevel::Warning,
                path: "$".to_owned(),
                message: "top-level value must be an object".to_owned(),
            }],
        );
    };

    input.version = object
        .get("version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .unwrap_or(domain::FORMAT_VERSION);

    if let Some(text) = object.get("t").and_then(Value::as_str) {
        input.title = Some(text.to_owned());
    }
    if let Some(text) = object.get("s").and_then(Value::as_str) {
        input.summary = Some(text.to_owned());
    }
    if let Some(theme) = object
        .get("theme")
        .and_then(Value::as_str)
        .filter(|item| is_theme(item))
    {
        input.theme = Some(theme.to_owned());
    }
    if let Some(density) = object
        .get("density")
        .and_then(Value::as_str)
        .filter(|item| is_density(item))
    {
        input.density = Some(density.to_owned());
    }
    if let Some(emphasis) = object
        .get("emphasis")
        .and_then(Value::as_str)
        .filter(|item| is_emphasis(item))
    {
        input.emphasis = Some(emphasis.to_owned());
    }

    let dictionaries = read_dictionaries(object.get("dict"));
    let mut warnings = Vec::new();
    let mut ref_alerts = Vec::new();
    let mut skipped_view_alerts = Vec::new();
    let mut dataset_metas = Vec::<CompactDatasetMeta>::new();

    if let Some(datasets) = object.get("d").and_then(Value::as_array) {
        for (dataset_index, dataset_value) in datasets.iter().enumerate() {
            let Some(tuple) = dataset_value.as_array() else {
                continue;
            };
            let Some(dataset_id) = tuple.first().and_then(Value::as_str) else {
                continue;
            };

            if tuple.get(1).and_then(Value::as_str) == Some("ref") {
                let reference = tuple.get(2).and_then(Value::as_str).unwrap_or_default();
                input.datasets.insert(
                    dataset_id.to_owned(),
                    Dataset {
                        columns: vec![Column {
                            key: "reference".to_owned(),
                            label: Some("Reference".to_owned()),
                            column_type: Some(domain::COLUMN_TYPE_STRING.to_owned()),
                            unit: None,
                            description: None,
                        }],
                        rows: Vec::new(),
                    },
                );
                dataset_metas.push(CompactDatasetMeta {
                    id: dataset_id.to_owned(),
                    columns: vec![CompactColumnMeta {
                        key: "reference".to_owned(),
                        type_code: TYPE_CODE_STRING.to_owned(),
                    }],
                    materialized: false,
                });
                warnings.push(Finding {
                    level: FindingLevel::Warning,
                    path: format!("$.d[{dataset_index}][2]"),
                    message: format!("dataset '{dataset_id}' uses external ref '{reference}'; fallback preview cannot resolve source rows"),
                });
                ref_alerts.push(Alert {
                    level: domain::ALERT_LEVEL_INFO.to_owned(),
                    title: Some("External data reference".to_owned()),
                    content: format!("Dataset '{dataset_id}' uses an external data reference; resolve it in the host UI tool before rendering data rows."),
                });
                continue;
            }

            let (columns_value, rows_value, is_column_major) =
                if tuple.get(1).and_then(Value::as_str) == Some("cols") {
                    (tuple.get(2), tuple.get(3), true)
                } else {
                    (tuple.get(1), tuple.get(2), false)
                };
            let (dataset, columns) = normalize_compact_dataset(
                columns_value
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                rows_value
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                is_column_major,
                &dictionaries,
            );
            input.datasets.insert(dataset_id.to_owned(), dataset);
            dataset_metas.push(CompactDatasetMeta {
                id: dataset_id.to_owned(),
                columns,
                materialized: true,
            });
        }
    }

    input.metrics = normalize_compact_metrics(object.get("m"));
    input.insights = normalize_compact_string_list(object.get("i"));
    input.assumptions = normalize_compact_string_list(object.get("as"));
    input.markdown = normalize_compact_markdown(object.get("md"));
    input.views = normalize_compact_views(
        object.get("v"),
        input.version,
        &dataset_metas,
        &mut skipped_view_alerts,
        &mut warnings,
    );
    input.alerts = normalize_compact_alerts(object.get("a"));
    input.alerts.extend(ref_alerts);
    input.alerts.extend(skipped_view_alerts);

    (input, warnings)
}

fn is_theme(value: &str) -> bool {
    domain::THEMES.contains(&value)
}

fn is_density(value: &str) -> bool {
    domain::DENSITIES.contains(&value)
}

fn is_emphasis(value: &str) -> bool {
    domain::EMPHASES.contains(&value)
}
