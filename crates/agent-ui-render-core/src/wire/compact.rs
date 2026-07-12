use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    diagnostic::{Finding, FindingLevel},
    domain::{
        self, Alert, Column, Dataset, MarkdownSection, Metric, MetricDelta, Primitive, Report,
        ViewIntent,
    },
};

pub const VERSION: u64 = 1;
pub const TOP_LEVEL_KEYS: &[&str] = &[
    "version", "t", "s", "theme", "density", "emphasis", "d", "m", "v", "a", "md", "dict", "i",
    "as",
];

pub const TYPE_CODE_STRING: &str = "s";
pub const TYPE_CODE_NUMBER: &str = "n";
pub const TYPE_CODE_CURRENCY: &str = "cur";
pub const TYPE_CODE_PERCENT: &str = "pct";
pub const TYPE_CODE_DATE: &str = "d";
pub const TYPE_CODE_DATETIME: &str = "dt";
pub const TYPE_CODE_BOOLEAN: &str = "b";
pub const TYPE_CODE_DICT_PREFIX: &str = "dict:";
pub const BASE_TYPE_CODES: &[&str] = &[
    TYPE_CODE_STRING,
    TYPE_CODE_NUMBER,
    TYPE_CODE_CURRENCY,
    TYPE_CODE_PERCENT,
    TYPE_CODE_DATE,
    TYPE_CODE_DATETIME,
    TYPE_CODE_BOOLEAN,
];
pub const DELTA_FORMAT_CODES: &[&str] = &[TYPE_CODE_NUMBER, TYPE_CODE_PERCENT];

pub const VIEW_CODE_OVERVIEW: &str = "o";
pub const VIEW_CODE_RECORDS: &str = "r";
pub const VIEW_CODE_TREND: &str = "t";
pub const VIEW_CODE_COMPARISON: &str = "b";
pub const VIEW_CODE_DISTRIBUTION: &str = "d";
pub const VIEW_CODE_COMPOSITION: &str = "p";
pub const VIEW_CODE_RELATIONSHIP: &str = "s";
pub const VIEW_CODES: &[&str] = &[
    VIEW_CODE_OVERVIEW,
    VIEW_CODE_RECORDS,
    VIEW_CODE_TREND,
    VIEW_CODE_COMPARISON,
    VIEW_CODE_DISTRIBUTION,
    VIEW_CODE_COMPOSITION,
    VIEW_CODE_RELATIONSHIP,
];
pub const SIMPLE_VIEW_CODES: &[&str] = &[VIEW_CODE_OVERVIEW, VIEW_CODE_RECORDS];
pub const MEASURE_VIEW_CODES: &[&str] = &[
    VIEW_CODE_TREND,
    VIEW_CODE_COMPARISON,
    VIEW_CODE_COMPOSITION,
    VIEW_CODE_RELATIONSHIP,
];

pub const ALERT_CODE_INFO: &str = "i";
pub const ALERT_CODE_SUCCESS: &str = "s";
pub const ALERT_CODE_WARNING: &str = "w";
pub const ALERT_CODE_ERROR: &str = "e";
pub const ALERT_CODE_CRITICAL: &str = "c";
pub const ALERT_LEVEL_CODES: &[&str] = &[
    ALERT_CODE_INFO,
    ALERT_CODE_SUCCESS,
    ALERT_CODE_WARNING,
    ALERT_CODE_ERROR,
    ALERT_CODE_CRITICAL,
];

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

fn normalize_compact_dataset(
    compact_columns: Vec<Value>,
    raw_data: Vec<Value>,
    is_column_major: bool,
    dictionaries: &BTreeMap<String, Vec<String>>,
) -> (Dataset, Vec<CompactColumnMeta>) {
    let metas: Vec<CompactColumnMeta> = compact_columns
        .iter()
        .enumerate()
        .map(compact_column_meta)
        .collect();
    let columns: Vec<Column> = compact_columns
        .iter()
        .enumerate()
        .map(normalize_compact_column)
        .collect();
    let raw_rows = if is_column_major {
        transpose_columns(raw_data)
    } else {
        raw_data
    };
    let rows = raw_rows
        .iter()
        .filter_map(Value::as_array)
        .map(|row| {
            columns
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    normalize_cell(
                        row.get(index).cloned().unwrap_or(Value::Null),
                        metas
                            .get(index)
                            .map_or(TYPE_CODE_STRING, |meta| meta.type_code.as_str()),
                        dictionaries,
                    )
                })
                .collect()
        })
        .collect();
    (Dataset { columns, rows }, metas)
}

fn compact_column_meta((index, value): (usize, &Value)) -> CompactColumnMeta {
    let tuple = value.as_array();
    CompactColumnMeta {
        key: tuple
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .map_or_else(|| format!("column_{}", index + 1), ToOwned::to_owned),
        type_code: tuple
            .and_then(|items| items.get(1))
            .and_then(Value::as_str)
            .unwrap_or(TYPE_CODE_STRING)
            .to_owned(),
    }
}

fn normalize_compact_column((index, value): (usize, &Value)) -> Column {
    let tuple = value.as_array();
    let key = tuple
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .map_or_else(|| format!("column_{}", index + 1), ToOwned::to_owned);
    let type_code = tuple
        .and_then(|items| items.get(1))
        .and_then(Value::as_str)
        .unwrap_or(TYPE_CODE_STRING);
    let unit = tuple
        .and_then(|items| items.get(2))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let label = tuple
        .and_then(|items| items.get(3))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| Some(titleize_key(&key)));
    Column {
        key,
        label,
        column_type: Some(normalize_type_code(type_code)),
        unit,
        description: None,
    }
}

fn normalize_compact_metrics(value: Option<&Value>) -> Vec<Metric> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|metric| {
            let tuple = metric.as_array()?;
            let label = tuple.first()?.as_str()?.to_owned();
            let value = tuple
                .get(1)
                .filter(|item| is_primitive(item))
                .cloned()
                .unwrap_or(Value::Null);
            let format = tuple
                .get(2)
                .and_then(Value::as_str)
                .map(normalize_metric_format);
            let unit = tuple.get(3).and_then(Value::as_str).map(ToOwned::to_owned);
            Some(Metric {
                label,
                value,
                format,
                unit,
                delta: normalize_compact_metric_delta(tuple.get(4)),
            })
        })
        .collect()
}

fn normalize_compact_metric_delta(value: Option<&Value>) -> Option<MetricDelta> {
    let value = value?;
    let (number, format_code) = if let Some(number) = value.as_f64() {
        (number, None)
    } else {
        let tuple = value.as_array()?;
        (
            tuple.first()?.as_f64()?,
            tuple.get(1).and_then(Value::as_str),
        )
    };
    let format = format_code
        .filter(|code| DELTA_FORMAT_CODES.contains(code))
        .map(normalize_metric_format);
    let direction = if number > 0.0 {
        domain::DELTA_DIRECTION_UP
    } else if number < 0.0 {
        domain::DELTA_DIRECTION_DOWN
    } else {
        domain::DELTA_DIRECTION_FLAT
    };
    Some(MetricDelta {
        label: Some(delta_label(number, format.as_deref())),
        value: number,
        format,
        direction: Some(direction.to_owned()),
    })
}

fn delta_label(value: f64, format: Option<&str>) -> String {
    let magnitude = if format == Some(domain::COLUMN_TYPE_PERCENT) {
        format!("{:.1}%", value.abs() * 100.0)
    } else if value.abs().fract() < f64::EPSILON {
        format!("{:.0}", value.abs())
    } else {
        format!("{:.2}", value.abs())
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    };
    if value > 0.0 {
        format!("+{magnitude}")
    } else if value < 0.0 {
        format!("-{magnitude}")
    } else {
        magnitude
    }
}

fn normalize_compact_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_compact_views(
    value: Option<&Value>,
    version: u32,
    dataset_metas: &[CompactDatasetMeta],
    skipped_alerts: &mut Vec<Alert>,
    warnings: &mut Vec<Finding>,
) -> Vec<ViewIntent> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, view)| {
            let code = view
                .as_array()
                .and_then(|tuple| tuple.first())
                .and_then(Value::as_str);
            let normalized = if version == domain::FORMAT_VERSION_V2
                && code.is_some_and(|item| !is_view_code(item))
            {
                let metas = dataset_metas
                    .iter()
                    .map(|dataset| crate::wire::v2::DatasetMeta {
                        id: dataset.id.clone(),
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
                match crate::wire::v2::normalize_view(view, index, &metas) {
                    Ok(view) => Some(view),
                    Err(message) => {
                        warnings.push(Finding {
                            level: FindingLevel::Warning,
                            path: format!("$.v[{index}]"),
                            message,
                        });
                        None
                    }
                }
            } else {
                normalize_compact_view(view, index, dataset_metas, warnings)
            };
            if normalized.is_none() {
                skipped_alerts.push(Alert {
                    level: domain::ALERT_LEVEL_WARNING.to_owned(),
                    title: Some("Skipped view".to_owned()),
                    content: format!("View {index} could not be mapped to valid normalized columns and was skipped."),
                });
            }
            normalized
        })
        .collect()
}

fn normalize_compact_view(
    value: &Value,
    index: usize,
    dataset_metas: &[CompactDatasetMeta],
    warnings: &mut Vec<Finding>,
) -> Option<ViewIntent> {
    let tuple = value.as_array()?;
    let code = tuple.first()?.as_str()?;
    let data_index = tuple.get(1)?.as_u64()? as usize;
    let dataset = dataset_metas.get(data_index)?;
    let data = dataset.id.clone();

    if code == VIEW_CODE_RECORDS {
        let columns = compact_column_keys(tuple.get(2), &dataset.columns);
        return Some(ViewIntent {
            intent: domain::VIEW_INTENT_PRECISE_RECORDS.to_owned(),
            data,
            x: None,
            measures: None,
            dimensions: None,
            columns: (!columns.is_empty()).then_some(columns),
            priority: None,
            title: None,
            chart: None,
            datasets: None,
            spec: None,
        });
    }
    if code == VIEW_CODE_OVERVIEW {
        return Some(ViewIntent {
            intent: domain::VIEW_INTENT_OVERVIEW.to_owned(),
            data,
            x: None,
            measures: None,
            dimensions: None,
            columns: None,
            priority: None,
            title: None,
            chart: None,
            datasets: None,
            spec: None,
        });
    }

    let x = tuple
        .get(2)
        .and_then(Value::as_u64)
        .and_then(|item| column_key_at(&dataset.columns, item as usize));
    let measures: Vec<String> = tuple
        .get(3)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.as_u64()
                .and_then(|index| column_key_at(&dataset.columns, index as usize))
        })
        .collect();

    if code == VIEW_CODE_DISTRIBUTION {
        let x = x?;
        return Some(ViewIntent {
            intent: domain::VIEW_INTENT_DISTRIBUTION.to_owned(),
            data,
            x: Some(x),
            measures: (!measures.is_empty()).then_some(measures),
            dimensions: None,
            columns: None,
            priority: None,
            title: None,
            chart: None,
            datasets: None,
            spec: None,
        });
    }

    let intent = view_intent_for_code(code)?;
    let x = x?;
    let chart_measures: Vec<String> = if code == VIEW_CODE_RELATIONSHIP {
        measures
            .into_iter()
            .filter(|measure| measure != &x)
            .collect()
    } else {
        measures
    };
    if chart_measures.is_empty() {
        warnings.push(Finding {
            level: FindingLevel::Warning,
            path: format!("$.v[{index}]"),
            message: format!(
                "view {index} with code '{code}' could not map indexes to required column keys"
            ),
        });
        return None;
    }
    Some(ViewIntent {
        intent: intent.to_owned(),
        data,
        x: Some(x),
        measures: Some(chart_measures),
        dimensions: None,
        columns: None,
        priority: None,
        title: None,
        chart: None,
        datasets: None,
        spec: None,
    })
}

fn compact_column_keys(value: Option<&Value>, columns: &[CompactColumnMeta]) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.as_u64()
                .and_then(|index| column_key_at(columns, index as usize))
        })
        .fold(Vec::new(), |mut keys, key| {
            if !keys.contains(&key) {
                keys.push(key);
            }
            keys
        })
}

fn normalize_compact_alerts(value: Option<&Value>) -> Vec<Alert> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|alert| {
            let tuple = alert.as_array()?;
            let level = tuple
                .first()?
                .as_str()
                .and_then(normalize_alert_level)?
                .to_owned();
            match tuple.len() {
                2 => Some(Alert {
                    level,
                    title: None,
                    content: tuple.get(1)?.as_str()?.to_owned(),
                }),
                3.. => Some(Alert {
                    level,
                    title: Some(tuple.get(1)?.as_str()?.to_owned()),
                    content: tuple.get(2)?.as_str()?.to_owned(),
                }),
                _ => None,
            }
        })
        .collect()
}

fn normalize_compact_markdown(value: Option<&Value>) -> Vec<MarkdownSection> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|section| {
            let tuple = section.as_array()?;
            match tuple.len() {
                1 => Some(MarkdownSection {
                    title: None,
                    content: tuple.first()?.as_str()?.to_owned(),
                }),
                2.. => Some(MarkdownSection {
                    title: Some(tuple.first()?.as_str()?.to_owned()),
                    content: tuple.get(1)?.as_str()?.to_owned(),
                }),
                _ => None,
            }
        })
        .collect()
}

fn read_dictionaries(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, entries)| {
            let strings: Vec<String> = entries
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect();
            Some((key.clone(), strings))
        })
        .collect()
}

fn transpose_columns(columns: Vec<Value>) -> Vec<Value> {
    let column_arrays: Vec<Vec<Value>> = columns
        .into_iter()
        .filter_map(|item| item.as_array().cloned())
        .collect();
    let row_count = column_arrays.first().map_or(0, Vec::len);
    (0..row_count)
        .map(|row_index| {
            Value::Array(
                column_arrays
                    .iter()
                    .map(|column| column.get(row_index).cloned().unwrap_or(Value::Null))
                    .collect(),
            )
        })
        .collect()
}

fn normalize_cell(
    value: Value,
    type_code: &str,
    dictionaries: &BTreeMap<String, Vec<String>>,
) -> Primitive {
    if let Some(dict_id) = type_code.strip_prefix(TYPE_CODE_DICT_PREFIX)
        && let Some(index) = value.as_u64()
    {
        return dictionaries
            .get(dict_id)
            .and_then(|entries| entries.get(index as usize))
            .map_or(Value::Null, |text| Value::String(text.clone()));
    }
    if is_primitive(&value) {
        value
    } else {
        Value::Null
    }
}

#[must_use]
pub fn normalize_type_code(type_code: &str) -> String {
    if type_code.starts_with(TYPE_CODE_DICT_PREFIX) {
        return domain::COLUMN_TYPE_STRING.to_owned();
    }
    match type_code {
        TYPE_CODE_NUMBER => domain::COLUMN_TYPE_NUMBER,
        TYPE_CODE_CURRENCY => domain::COLUMN_TYPE_CURRENCY,
        TYPE_CODE_PERCENT => domain::COLUMN_TYPE_PERCENT,
        TYPE_CODE_DATE => domain::COLUMN_TYPE_DATE,
        TYPE_CODE_DATETIME => domain::COLUMN_TYPE_DATETIME,
        TYPE_CODE_BOOLEAN => domain::COLUMN_TYPE_BOOLEAN,
        _ => domain::COLUMN_TYPE_STRING,
    }
    .to_owned()
}

#[must_use]
pub fn normalize_metric_format(type_code: &str) -> String {
    match type_code {
        TYPE_CODE_NUMBER => domain::COLUMN_TYPE_NUMBER,
        TYPE_CODE_CURRENCY => domain::COLUMN_TYPE_CURRENCY,
        TYPE_CODE_PERCENT => domain::COLUMN_TYPE_PERCENT,
        _ => domain::COLUMN_TYPE_STRING,
    }
    .to_owned()
}

#[must_use]
pub fn view_intent_for_code(code: &str) -> Option<&'static str> {
    match code {
        VIEW_CODE_OVERVIEW => Some(domain::VIEW_INTENT_OVERVIEW),
        VIEW_CODE_RECORDS => Some(domain::VIEW_INTENT_PRECISE_RECORDS),
        VIEW_CODE_TREND => Some(domain::VIEW_INTENT_TREND),
        VIEW_CODE_COMPARISON => Some(domain::VIEW_INTENT_COMPARISON),
        VIEW_CODE_DISTRIBUTION => Some(domain::VIEW_INTENT_DISTRIBUTION),
        VIEW_CODE_COMPOSITION => Some(domain::VIEW_INTENT_COMPOSITION),
        VIEW_CODE_RELATIONSHIP => Some(domain::VIEW_INTENT_RELATIONSHIP),
        _ => None,
    }
}

#[must_use]
pub fn normalize_alert_level(code: &str) -> Option<&'static str> {
    match code {
        ALERT_CODE_INFO => Some(domain::ALERT_LEVEL_INFO),
        ALERT_CODE_SUCCESS => Some(domain::ALERT_LEVEL_SUCCESS),
        ALERT_CODE_WARNING => Some(domain::ALERT_LEVEL_WARNING),
        ALERT_CODE_ERROR => Some(domain::ALERT_LEVEL_ERROR),
        ALERT_CODE_CRITICAL => Some(domain::ALERT_LEVEL_CRITICAL),
        _ => None,
    }
}

#[must_use]
pub fn is_view_code(value: &str) -> bool {
    VIEW_CODES.contains(&value)
}

#[must_use]
pub fn is_simple_view_code(value: &str) -> bool {
    SIMPLE_VIEW_CODES.contains(&value)
}

#[must_use]
pub fn is_measure_view_code(value: &str) -> bool {
    MEASURE_VIEW_CODES.contains(&value)
}

#[must_use]
pub fn is_alert_level_code(value: &str) -> bool {
    ALERT_LEVEL_CODES.contains(&value)
}

#[must_use]
pub fn is_base_or_dict_type_code(value: &str) -> bool {
    BASE_TYPE_CODES.contains(&value) || value.starts_with(TYPE_CODE_DICT_PREFIX)
}

fn column_key_at(columns: &[CompactColumnMeta], index: usize) -> Option<String> {
    columns.get(index).map(|column| column.key.clone())
}

fn titleize_key(key: &str) -> String {
    key.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

fn is_primitive(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
}
