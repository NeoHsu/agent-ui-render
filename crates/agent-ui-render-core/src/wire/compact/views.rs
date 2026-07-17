use serde_json::Value;

use crate::{
    diagnostic::{Finding, FindingLevel},
    domain::{self, Alert, ViewIntent},
};

use super::{
    CompactColumnMeta, CompactDatasetMeta, VIEW_CODE_DISTRIBUTION, VIEW_CODE_OVERVIEW,
    VIEW_CODE_RECORDS, VIEW_CODE_RELATIONSHIP, is_view_code, view_intent_for_code,
};

pub(super) fn normalize_compact_views(
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

fn column_key_at(columns: &[CompactColumnMeta], index: usize) -> Option<String> {
    columns.get(index).map(|column| column.key.clone())
}
