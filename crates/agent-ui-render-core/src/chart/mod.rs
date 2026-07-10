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
pub fn bar_orientation_for_view(view: &ViewIntent, dataset: &Dataset) -> &'static str {
    if view.intent != crate::domain::VIEW_INTENT_COMPARISON
        || !has_compact_temporal_categories(dataset, view)
    {
        return "horizontal";
    }
    if has_compatible_measures(dataset, view) {
        "vertical"
    } else {
        "horizontal"
    }
}

fn has_compact_temporal_categories(dataset: &Dataset, view: &ViewIntent) -> bool {
    if !(2..=8).contains(&dataset.rows.len()) {
        return false;
    }
    let Some(x_index) = column_index(dataset, view.x.as_deref()) else {
        return false;
    };
    if matches!(
        dataset.columns[x_index].column_type.as_deref(),
        Some(crate::domain::COLUMN_TYPE_DATE) | Some(crate::domain::COLUMN_TYPE_DATETIME)
    ) {
        return true;
    }
    dataset
        .rows
        .iter()
        .all(|row| row.get(x_index).is_some_and(is_temporal_category))
}

fn has_compatible_measures(dataset: &Dataset, view: &ViewIntent) -> bool {
    measure_keys(dataset, view)
        .iter()
        .filter_map(|key| column_index(dataset, Some(key)))
        .map(|index| {
            let column = &dataset.columns[index];
            format!(
                "{}:{}",
                column.column_type.as_deref().unwrap_or_default(),
                column.unit.as_deref().unwrap_or_default()
            )
        })
        .collect::<BTreeSet<_>>()
        .len()
        == 1
}

fn is_temporal_category(value: &Value) -> bool {
    let text = value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned);
    is_temporal_label(text.trim())
}

fn is_temporal_label(text: &str) -> bool {
    let upper = text.trim().to_ascii_uppercase();
    let first = upper
        .split(|character: char| character.is_whitespace() || ",/'’-".contains(character))
        .next()
        .unwrap_or_default();
    is_year(first)
        || is_iso_period(&upper)
        || matches!(first, "Q1" | "Q2" | "Q3" | "Q4" | "1Q" | "2Q" | "3Q" | "4Q")
        || is_week_period(&upper)
        || matches!(
            first,
            "JAN"
                | "JANUARY"
                | "FEB"
                | "FEBRUARY"
                | "MAR"
                | "MARCH"
                | "APR"
                | "APRIL"
                | "MAY"
                | "JUN"
                | "JUNE"
                | "JUL"
                | "JULY"
                | "AUG"
                | "AUGUST"
                | "SEP"
                | "SEPT"
                | "SEPTEMBER"
                | "OCT"
                | "OCTOBER"
                | "NOV"
                | "NOVEMBER"
                | "DEC"
                | "DECEMBER"
        )
}

fn is_year(text: &str) -> bool {
    text.len() == 4
        && text
            .parse::<u16>()
            .is_ok_and(|year| (1900..=2099).contains(&year))
}

fn is_iso_period(text: &str) -> bool {
    text.len() >= 7
        && text
            .as_bytes()
            .get(4)
            .is_some_and(|byte| matches!(byte, b'-' | b'/'))
        && text.get(..4).is_some_and(is_year)
}

fn is_week_period(text: &str) -> bool {
    let compact = text.replace("WEEK", "W").replace(' ', "");
    compact
        .strip_prefix('W')
        .and_then(|rest| {
            rest.split(|character| [',', '/', '-'].contains(&character))
                .next()
        })
        .is_some_and(|week| {
            week.parse::<u8>()
                .is_ok_and(|value| (1..=53).contains(&value))
        })
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
