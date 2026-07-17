mod charts;
mod helpers;
mod layout;
mod options;
mod spec_options;

use charts::build_chart_spec;
use helpers::{
    chart_name, column, datum_field, deduplicate, ensure_numeric, field_def, remove_data,
    remove_schema, replace_first_quantitative_field, series_legend_label_expr, titleize,
};
use layout::normalize_layout;
use options::{
    expect_len, option_bins, option_bool, option_string, optional_index, optional_indexes, options,
    required_index, required_indexes, validate_options,
};
use spec_options::{add_interaction, add_title, with_common_options};

use serde_json::Value;

use crate::domain::{self, ViewIntent};

pub const VERSION: u64 = 2;
pub const VEGA_LITE_SCHEMA: &str = "https://vega.github.io/schema/vega-lite/v6.4.3.json";

pub const CHART_CODES: &[&str] = &[
    "ln", "ar", "tr", "reg", "bar", "gantt", "water", "bullet", "range", "hist", "den", "dot",
    "box", "box5", "qq", "sc", "par", "tri", "heat", "mos", "pie", "don", "rad", "err", "band",
    "candle", "text", "tick", "rule",
];

pub const LAYOUT_CODES: &[&str] = &["layer", "facet", "concat", "repeat"];
pub const NORMALIZED_CHARTS: &[&str] = &[
    "line",
    "area",
    "trail",
    "regression",
    "bar",
    "gantt",
    "waterfall",
    "bullet",
    "ranged-dot",
    "histogram",
    "density",
    "dot",
    "boxplot",
    "qq",
    "scatter",
    "parallel-coordinates",
    "ternary",
    "heatmap",
    "mosaic",
    "pie",
    "donut",
    "radial",
    "errorbar",
    "errorband",
    "candlestick",
    "text",
    "tick",
    "rule",
    "layer",
    "facet",
    "concat",
    "repeat",
];

#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub key: String,
    pub type_code: String,
}

#[derive(Debug, Clone)]
pub struct DatasetMeta {
    pub id: String,
    pub columns: Vec<ColumnMeta>,
    pub materialized: bool,
}

pub fn normalize_view(
    value: &Value,
    index: usize,
    datasets: &[DatasetMeta],
) -> Result<ViewIntent, String> {
    let tuple = value
        .as_array()
        .ok_or_else(|| "view must be a tuple array".to_owned())?;
    let code = tuple
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| "view code must be a string".to_owned())?;
    if LAYOUT_CODES.contains(&code) {
        return normalize_layout(tuple, index, datasets);
    }
    if !CHART_CODES.contains(&code) {
        return Err(format!("unsupported v2 chart code '{code}'"));
    }
    let options = options(tuple)?;
    validate_options(options)?;
    let dataset_index = required_index(tuple, 1, "dataset")?;
    let dataset = datasets
        .get(dataset_index)
        .ok_or_else(|| format!("chart references missing dataset index {dataset_index}"))?;
    if !dataset.materialized {
        return Err(format!(
            "chart '{code}' requires materialized columns; dataset '{}' is an external reference",
            dataset.id
        ));
    }
    let spec = build_chart_spec(code, tuple, dataset, options)?;
    Ok(chart_view(
        chart_name(code),
        vec![dataset.id.clone()],
        option_string(options, "t").map(ToOwned::to_owned),
        spec,
    ))
}

fn chart_view(
    chart: &str,
    datasets: Vec<String>,
    title: Option<String>,
    spec: Value,
) -> ViewIntent {
    ViewIntent {
        intent: domain::VIEW_INTENT_CHART.to_owned(),
        data: datasets.first().cloned().unwrap_or_default(),
        x: None,
        measures: None,
        dimensions: None,
        columns: None,
        priority: None,
        title,
        chart: Some(chart.to_owned()),
        datasets: Some(datasets),
        spec: Some(spec),
    }
}
