use std::collections::BTreeSet;

use serde_json::Value;

use crate::domain::{Dataset, ViewIntent};

pub const MAX_PIE_CATEGORIES: usize = 5;

#[must_use]
pub fn chart_kind_for_view(view: &ViewIntent, dataset: &Dataset) -> &'static str {
    match view.intent.as_str() {
        crate::domain::VIEW_INTENT_TREND => "line",
        crate::domain::VIEW_INTENT_RELATIONSHIP => "scatter",
        crate::domain::VIEW_INTENT_COMPOSITION
            if can_use_pie_chart(dataset, view, MAX_PIE_CATEGORIES) =>
        {
            "pie"
        }
        _ => "bar",
    }
}

#[must_use]
pub fn can_use_pie_chart(dataset: &Dataset, view: &ViewIntent, max_categories: usize) -> bool {
    if view.intent != crate::domain::VIEW_INTENT_COMPOSITION {
        return false;
    }
    let Some(x_index) = column_index(dataset, view.x.as_deref()) else {
        return false;
    };
    let measure = measure_keys(dataset, view).into_iter().next();
    let Some(y_index) = column_index(dataset, measure.as_deref()) else {
        return false;
    };
    if dataset.rows.is_empty() {
        return false;
    }
    let distinct = dataset
        .rows
        .iter()
        .map(|row| row.get(x_index).map_or("null".to_owned(), Value::to_string))
        .collect::<BTreeSet<_>>()
        .len();
    if distinct == 0 || distinct > max_categories {
        return false;
    }
    let total: f64 = dataset
        .rows
        .iter()
        .filter_map(|row| numeric_value(row, y_index))
        .map(|value| value.max(0.0))
        .sum();
    total > 0.0
}

#[must_use]
pub fn column_index(dataset: &Dataset, key: Option<&str>) -> Option<usize> {
    let key = key?;
    dataset.columns.iter().position(|column| column.key == key)
}

#[must_use]
pub fn first_numeric_column(dataset: &Dataset) -> Option<String> {
    dataset
        .columns
        .iter()
        .find(|column| is_numeric_column_type(column.column_type.as_deref()))
        .map(|column| column.key.clone())
}

#[must_use]
pub fn first_numeric_columns(dataset: &Dataset, count: usize) -> Vec<String> {
    dataset
        .columns
        .iter()
        .filter(|column| is_numeric_column_type(column.column_type.as_deref()))
        .take(count)
        .map(|column| column.key.clone())
        .collect()
}

#[must_use]
pub fn first_non_measure_column(dataset: &Dataset, measures: &[String]) -> Option<String> {
    dataset
        .columns
        .iter()
        .find(|column| !measures.contains(&column.key))
        .map(|column| column.key.clone())
        .or_else(|| dataset.columns.first().map(|column| column.key.clone()))
}

#[must_use]
pub fn measure_keys(dataset: &Dataset, view: &ViewIntent) -> Vec<String> {
    if let Some(measures) = &view.measures
        && !measures.is_empty()
    {
        return measures.clone();
    }
    first_numeric_column(dataset).into_iter().collect()
}

#[must_use]
pub fn numeric_value(row: &[Value], index: usize) -> Option<f64> {
    row.get(index)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn is_numeric_column_type(column_type: Option<&str>) -> bool {
    matches!(
        column_type,
        Some(
            crate::domain::COLUMN_TYPE_NUMBER
                | crate::domain::COLUMN_TYPE_CURRENCY
                | crate::domain::COLUMN_TYPE_PERCENT
        )
    )
}

#[must_use]
pub fn extent(values: &[f64]) -> (f64, f64) {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (min - max).abs() < f64::EPSILON {
        (min - 1.0, max + 1.0)
    } else {
        let pad = (max - min) * 0.08;
        (min - pad, max + pad)
    }
}
