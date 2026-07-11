use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

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

const OPTION_KEYS: &[&str] = &[
    "t", "or", "st", "ag", "bn", "ip", "mode", "pt", "lb", "lg", "tip", "sort", "top", "zero",
    "shape", "jitter", "sel", "resolve",
];

trait InsertProperty {
    fn insert(&mut self, key: String, value: Value);
}

impl InsertProperty for Value {
    fn insert(&mut self, key: String, value: Value) {
        if let Some(object) = self.as_object_mut() {
            object.insert(key, value);
        }
    }
}

const MODES: &[&str] = &[
    "slope",
    "bump",
    "stream",
    "horizon",
    "diverging",
    "strip",
    "rug",
    "pyramid",
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

fn normalize_layout(
    tuple: &[Value],
    index: usize,
    datasets: &[DatasetMeta],
) -> Result<ViewIntent, String> {
    let code = tuple[0].as_str().unwrap_or_default();
    let options = options(tuple)?;
    validate_options(options)?;
    match code {
        "layer" => {
            expect_len(tuple, 2, 3, "layer")?;
            let children = tuple
                .get(1)
                .and_then(Value::as_array)
                .ok_or_else(|| "layer children must be an array".to_owned())?;
            if children.is_empty() {
                return Err("layer must contain at least one chart".to_owned());
            }
            let mut layers = Vec::new();
            let mut refs = Vec::new();
            for (child_index, child) in children.iter().enumerate() {
                let child = normalize_view(child, index * 100 + child_index, datasets)?;
                if child
                    .chart
                    .as_deref()
                    .is_some_and(|item| LAYOUT_CODES.contains(&item))
                {
                    return Err("nested layout inside layer is not supported".to_owned());
                }
                let mut spec = child.spec.unwrap_or_else(|| json!({}));
                remove_schema(&mut spec);
                layers.push(spec);
                refs.extend(child.datasets.unwrap_or_default());
            }
            deduplicate(&mut refs);
            let mut spec = json!({
                "$schema": VEGA_LITE_SCHEMA,
                "layer": layers,
                "resolve": {"scale": {"color": option_string(options, "resolve").unwrap_or("shared")}}
            });
            add_title(&mut spec, option_string(options, "t"));
            Ok(chart_view(
                "layer",
                refs,
                option_string(options, "t").map(ToOwned::to_owned),
                spec,
            ))
        }
        "facet" => {
            expect_len(tuple, 3, 5, "facet")?;
            let child_value = tuple
                .get(1)
                .ok_or_else(|| "facet requires a child chart".to_owned())?;
            let child = normalize_view(child_value, index * 100, datasets)?;
            let data_id = child.data.clone();
            let dataset = datasets
                .iter()
                .find(|item| item.id == data_id)
                .ok_or_else(|| "facet child dataset is unavailable".to_owned())?;
            let row = optional_index(tuple, 2)?;
            let column = if tuple.get(3).is_some_and(Value::is_object) {
                None
            } else {
                optional_index(tuple, 3)?
            };
            if row.is_none() && column.is_none() {
                return Err("facet requires a row or column field".to_owned());
            }
            let mut facet = Map::new();
            if let Some(row) = row {
                facet.insert("row".to_owned(), field_def(dataset, row, false)?);
            }
            if let Some(column) = column {
                facet.insert("column".to_owned(), field_def(dataset, column, false)?);
            }
            let mut child_spec = child.spec.unwrap_or_else(|| json!({}));
            remove_schema(&mut child_spec);
            remove_data(&mut child_spec);
            let mut spec = json!({
                "$schema": VEGA_LITE_SCHEMA,
                "data": {"name": data_id},
                "facet": facet,
                "spec": child_spec
            });
            add_title(&mut spec, option_string(options, "t"));
            Ok(chart_view(
                "facet",
                child.datasets.unwrap_or_else(|| vec![data_id]),
                option_string(options, "t").map(ToOwned::to_owned),
                spec,
            ))
        }
        "concat" => {
            expect_len(tuple, 3, 4, "concat")?;
            let direction = tuple
                .get(1)
                .and_then(Value::as_str)
                .filter(|item| matches!(*item, "h" | "v"))
                .ok_or_else(|| "concat direction must be 'h' or 'v'".to_owned())?;
            let children = tuple
                .get(2)
                .and_then(Value::as_array)
                .ok_or_else(|| "concat children must be an array".to_owned())?;
            if children.is_empty() {
                return Err("concat must contain at least one view".to_owned());
            }
            let mut specs = Vec::new();
            let mut refs = Vec::new();
            for (child_index, child) in children.iter().enumerate() {
                let child = normalize_view(child, index * 100 + child_index, datasets)?;
                let mut child_spec = child.spec.unwrap_or_else(|| json!({}));
                remove_schema(&mut child_spec);
                specs.push(child_spec);
                refs.extend(child.datasets.unwrap_or_default());
            }
            deduplicate(&mut refs);
            let key = if direction == "h" {
                "hconcat"
            } else {
                "vconcat"
            };
            let mut object = Map::new();
            object.insert("$schema".to_owned(), json!(VEGA_LITE_SCHEMA));
            object.insert(key.to_owned(), Value::Array(specs));
            let mut spec = Value::Object(object);
            add_title(&mut spec, option_string(options, "t"));
            Ok(chart_view(
                "concat",
                refs,
                option_string(options, "t").map(ToOwned::to_owned),
                spec,
            ))
        }
        "repeat" => {
            expect_len(tuple, 4, 5, "repeat")?;
            let child_value = tuple
                .get(1)
                .ok_or_else(|| "repeat requires a child chart".to_owned())?;
            let child = normalize_view(child_value, index * 100, datasets)?;
            let columns = tuple
                .get(2)
                .and_then(Value::as_array)
                .ok_or_else(|| "repeat columns must be an array".to_owned())?;
            if columns.is_empty() {
                return Err("repeat columns must not be empty".to_owned());
            }
            let direction = tuple
                .get(3)
                .and_then(Value::as_str)
                .filter(|item| matches!(*item, "h" | "v" | "grid"))
                .ok_or_else(|| "repeat direction must be h, v, or grid".to_owned())?;
            let dataset = datasets
                .iter()
                .find(|item| item.id == child.data)
                .ok_or_else(|| "repeat child dataset is unavailable".to_owned())?;
            let field_names = columns
                .iter()
                .map(|item| {
                    let index = item
                        .as_u64()
                        .ok_or_else(|| "repeat column index must be non-negative".to_owned())?
                        as usize;
                    Ok(column(dataset, index)?.key.clone())
                })
                .collect::<Result<Vec<_>, String>>()?;
            let mut child_spec = child.spec.unwrap_or_else(|| json!({}));
            remove_schema(&mut child_spec);
            replace_first_quantitative_field(&mut child_spec, &json!({"repeat": "repeat"}));
            let repeat = match direction {
                "h" => json!({"column": field_names}),
                "v" => json!({"row": field_names}),
                _ => json!(field_names),
            };
            let mut spec = json!({
                "$schema": VEGA_LITE_SCHEMA,
                "repeat": repeat,
                "spec": child_spec
            });
            add_title(&mut spec, option_string(options, "t"));
            Ok(chart_view(
                "repeat",
                child.datasets.unwrap_or_else(|| vec![child.data]),
                option_string(options, "t").map(ToOwned::to_owned),
                spec,
            ))
        }
        _ => Err(format!("unsupported layout code '{code}'")),
    }
}

fn build_chart_spec(
    code: &str,
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    let spec = match code {
        "ln" => line_or_area_spec(tuple, dataset, options, false)?,
        "ar" => line_or_area_spec(tuple, dataset, options, true)?,
        "tr" => trail_spec(tuple, dataset, options)?,
        "reg" => regression_spec(tuple, dataset, options)?,
        "bar" => bar_spec(tuple, dataset, options)?,
        "gantt" => gantt_spec(tuple, dataset, options)?,
        "water" => waterfall_spec(tuple, dataset, options)?,
        "bullet" => bullet_spec(tuple, dataset, options)?,
        "range" => range_spec(tuple, dataset, options)?,
        "hist" => histogram_spec(tuple, dataset, options)?,
        "den" => density_spec(tuple, dataset, options)?,
        "dot" => dot_spec(tuple, dataset, options)?,
        "box" => boxplot_spec(tuple, dataset, options)?,
        "box5" => boxplot_summary_spec(tuple, dataset, options)?,
        "qq" => qq_spec(tuple, dataset, options)?,
        "sc" => scatter_spec(tuple, dataset, options)?,
        "par" => parallel_spec(tuple, dataset, options)?,
        "tri" => ternary_spec(tuple, dataset, options)?,
        "heat" => heatmap_spec(tuple, dataset, options)?,
        "mos" => mosaic_spec(tuple, dataset, options)?,
        "pie" | "don" | "rad" => arc_spec(code, tuple, dataset, options)?,
        "err" => error_spec(tuple, dataset, options, false)?,
        "band" => error_spec(tuple, dataset, options, true)?,
        "candle" => candlestick_spec(tuple, dataset, options)?,
        "text" => text_spec(tuple, dataset, options)?,
        "tick" => tick_spec(tuple, dataset, options)?,
        "rule" => rule_spec(tuple, dataset, options)?,
        _ => return Err(format!("unsupported chart code '{code}'")),
    };
    Ok(with_common_options(spec, options))
}

fn line_or_area_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
    area: bool,
) -> Result<Value, String> {
    expect_len(tuple, 4, 5, if area { "area" } else { "line" })?;
    let x = required_index(tuple, 2, "x")?;
    let ys = required_indexes(tuple, 3, "measures")?;
    ensure_numeric(dataset, &ys, "line/area measures")?;
    let mark_type = if area { "area" } else { "line" };
    let mut mark = json!({"type": mark_type, "tooltip": option_bool(options, "tip", true)});
    if let Some(interpolate) = option_string(options, "ip") {
        mark["interpolate"] = json!(interpolate);
    }
    if !area && option_bool(options, "pt", false) {
        mark["point"] = json!(true);
    }
    let (mut encoding, mut transform) = multi_measure_encoding(dataset, x, &ys, options)?;
    if area && let Some(y) = encoding.get_mut("y").and_then(Value::as_object_mut) {
        let stack = match option_string(options, "st").unwrap_or("none") {
            "zero" => json!("zero"),
            "normalize" => json!("normalize"),
            "center" => json!("center"),
            _ => Value::Null,
        };
        y.insert("stack".to_owned(), stack);
    }
    match option_string(options, "mode") {
        Some("slope") => {
            transform.splice(
                0..0,
                [
                    json!({"window": [{"op": "row_number", "as": "__row"}]}),
                    json!({"joinaggregate": [{"op": "max", "field": "__row", "as": "__last"}]}),
                    json!({"filter": "datum.__row === 1 || datum.__row === datum.__last"}),
                ],
            );
        }
        Some("bump") if ys.len() > 1 => {
            transform.push(json!({
                "window": [{"op": "rank", "as": "__rank"}],
                "sort": [{"field": "__value", "order": "descending"}],
                "groupby": [column(dataset, x)?.key]
            }));
            encoding.insert(
                "y".to_owned(),
                json!({"field": "__rank", "type": "ordinal", "sort": "ascending", "title": "Rank"}),
            );
        }
        _ => {}
    }
    let mut spec = json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "mark": mark,
        "encoding": encoding
    });
    if !transform.is_empty() {
        spec["transform"] = Value::Array(transform);
    }
    if option_string(options, "mode") == Some("horizon") {
        spec["mark"]["opacity"] = json!(0.65);
    }
    Ok(spec)
}

fn trail_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 7, "trail")?;
    let x = required_index(tuple, 2, "x")?;
    let y = required_index(tuple, 3, "y")?;
    let width = required_index(tuple, 4, "width")?;
    ensure_numeric(dataset, &[y, width], "trail y/width")?;
    let color = optional_index(tuple, 5)?;
    let mut encoding = json!({
        "x": field_def(dataset, x, false)?,
        "y": field_def(dataset, y, true)?,
        "size": field_def(dataset, width, true)?
    });
    if let Some(color) = color {
        encoding.insert("color".to_owned(), field_def(dataset, color, false)?);
    }
    Ok(base_spec(
        dataset,
        json!({"type": "trail", "tooltip": true}),
        encoding,
    ))
}

fn regression_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 7, "regression")?;
    let x = required_index(tuple, 2, "x")?;
    let y = required_index(tuple, 3, "y")?;
    ensure_numeric(dataset, &[x, y], "regression x/y")?;
    let color = optional_index(tuple, 4)?;
    let mode = tuple
        .get(5)
        .or_else(|| tuple.get(4))
        .and_then(Value::as_str)
        .filter(|item| matches!(*item, "linear" | "loess"))
        .ok_or_else(|| "regression mode must be 'linear' or 'loess'".to_owned())?;
    let x_key = &column(dataset, x)?.key;
    let y_key = &column(dataset, y)?.key;
    let mut point_encoding = json!({
        "x": field_def(dataset, x, true)?,
        "y": field_def(dataset, y, true)?
    });
    if let Some(color) = color {
        point_encoding.insert("color".to_owned(), field_def(dataset, color, false)?);
    }
    let transform = if mode == "loess" {
        json!({"loess": y_key, "on": x_key})
    } else {
        json!({"regression": y_key, "on": x_key})
    };
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "layer": [
            {"mark": {"type": "point", "opacity": 0.55, "tooltip": true}, "encoding": point_encoding},
            {"transform": [transform], "mark": {"type": "line", "strokeWidth": 3}, "encoding": {
                "x": field_def(dataset, x, true)?,
                "y": field_def(dataset, y, true)?
            }}
        ]
    }))
}

fn bar_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 4, 5, "bar")?;
    let x = required_index(tuple, 2, "category")?;
    let ys = required_indexes(tuple, 3, "measures")?;
    ensure_numeric(dataset, &ys, "bar measures")?;
    let orientation = option_string(options, "or").unwrap_or("v");
    let stack = option_string(options, "st").unwrap_or("none");
    let (mut encoding, transform) = multi_measure_encoding(dataset, x, &ys, options)?;
    if orientation == "h" {
        let old_x = encoding.get("x").cloned().unwrap_or(Value::Null);
        let old_y = encoding.get("y").cloned().unwrap_or(Value::Null);
        encoding.insert("x".to_owned(), old_y);
        encoding.insert("y".to_owned(), old_x);
    }
    let quantitative_channel = if orientation == "h" { "x" } else { "y" };
    if let Some(definition) = encoding
        .get_mut(quantitative_channel)
        .and_then(Value::as_object_mut)
    {
        definition.insert(
            "stack".to_owned(),
            match stack {
                "zero" => json!("zero"),
                "normalize" => json!("normalize"),
                "center" => json!("center"),
                _ => Value::Null,
            },
        );
    }
    if ys.len() > 1 && stack == "none" {
        let offset = if orientation == "h" {
            "yOffset"
        } else {
            "xOffset"
        };
        encoding.insert(offset.to_owned(), json!({"field": "__series"}));
    }
    if option_string(options, "mode") == Some("diverging") {
        let value_field = if ys.len() > 1 {
            "__value".to_owned()
        } else {
            column(dataset, ys[0])?.key.clone()
        };
        encoding.insert(
            "color".to_owned(),
            json!({
                "condition": {"test": format!("{} < 0", datum_field(&value_field)), "value": "#b91c1c"},
                "value": "#15803d",
                "legend": null
            }),
        );
    }
    let mut spec = base_spec(
        dataset,
        json!({"type": "bar", "tooltip": option_bool(options, "tip", true)}),
        Value::Object(encoding),
    );
    if !transform.is_empty() {
        spec["transform"] = Value::Array(transform);
    }
    Ok(spec)
}

fn gantt_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 7, "gantt")?;
    let task = required_index(tuple, 2, "task")?;
    let start = required_index(tuple, 3, "start")?;
    let end = required_index(tuple, 4, "end")?;
    let group = optional_index(tuple, 5)?;
    let mut encoding = json!({
        "y": field_def(dataset, task, false)?,
        "x": field_def(dataset, start, false)?,
        "x2": {"field": column(dataset, end)?.key},
        "tooltip": [field_def(dataset, task, false)?, field_def(dataset, start, false)?, field_def(dataset, end, false)?]
    });
    if let Some(group) = group {
        encoding.insert("color".to_owned(), field_def(dataset, group, false)?);
    }
    Ok(base_spec(
        dataset,
        json!({"type": "bar", "cornerRadius": 3}),
        encoding,
    ))
}

fn waterfall_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 4, 5, "waterfall")?;
    let x = required_index(tuple, 2, "category")?;
    let y = required_index(tuple, 3, "value")?;
    ensure_numeric(dataset, &[y], "waterfall value")?;
    let y_key = &column(dataset, y)?.key;
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "transform": [
            {"window": [{"op": "sum", "field": y_key, "as": "__end"}]},
            {"calculate": format!("datum.__end - {}", datum_field(y_key)), "as": "__start"}
        ],
        "mark": {"type": "bar", "tooltip": true},
        "encoding": {
            "x": field_def(dataset, x, false)?,
            "y": {"field": "__start", "type": "quantitative"},
            "y2": {"field": "__end"},
            "color": {"condition": {"test": format!("{} < 0", datum_field(y_key)), "value": "#b91c1c"}, "value": "#15803d"}
        }
    }))
}

fn bullet_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 7, "bullet")?;
    let category = required_index(tuple, 2, "category")?;
    let actual = required_index(tuple, 3, "actual")?;
    let target = required_index(tuple, 4, "target")?;
    ensure_numeric(dataset, &[actual, target], "bullet values")?;
    let ranges = optional_indexes(tuple, 5)?;
    ensure_numeric(dataset, &ranges, "bullet ranges")?;
    let mut layers = Vec::new();
    for (index, range) in ranges.iter().rev().enumerate() {
        layers.push(json!({
            "mark": {"type": "bar", "opacity": 0.18 + index as f64 * 0.1, "height": 24},
            "encoding": {"x": field_def(dataset, *range, true)?, "y": field_def(dataset, category, false)?}
        }));
    }
    layers.push(json!({
        "mark": {"type": "bar", "height": 10, "tooltip": true},
        "encoding": {"x": field_def(dataset, actual, true)?, "y": field_def(dataset, category, false)?}
    }));
    layers.push(json!({
        "mark": {"type": "tick", "thickness": 3, "size": 28},
        "encoding": {"x": field_def(dataset, target, true)?, "y": field_def(dataset, category, false)?}
    }));
    Ok(json!({"$schema": VEGA_LITE_SCHEMA, "data": {"name": dataset.id}, "layer": layers}))
}

fn range_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 6, "range")?;
    let category = required_index(tuple, 2, "category")?;
    let start = required_index(tuple, 3, "start")?;
    let end = required_index(tuple, 4, "end")?;
    ensure_numeric(dataset, &[start, end], "range values")?;
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "layer": [
            {"mark": {"type": "rule", "strokeWidth": 4}, "encoding": {
                "y": field_def(dataset, category, false)?, "x": field_def(dataset, start, true)?, "x2": {"field": column(dataset, end)?.key}
            }},
            {"mark": {"type": "point", "filled": true, "size": 70}, "encoding": {
                "y": field_def(dataset, category, false)?, "x": field_def(dataset, start, true)?
            }},
            {"mark": {"type": "point", "filled": true, "size": 70}, "encoding": {
                "y": field_def(dataset, category, false)?, "x": field_def(dataset, end, true)?
            }}
        ]
    }))
}

fn histogram_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 3, 4, "histogram")?;
    let x = required_index(tuple, 2, "value")?;
    ensure_numeric(dataset, &[x], "histogram value")?;
    let maxbins = option_bins(options).map_or(30, |items| items[0]);
    Ok(base_spec(
        dataset,
        json!({"type": "bar", "tooltip": true}),
        json!({
            "x": {"field": column(dataset, x)?.key, "type": "quantitative", "bin": {"maxbins": maxbins}},
            "y": {"aggregate": "count", "type": "quantitative", "title": "Count"}
        }),
    ))
}

fn density_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 3, 5, "density")?;
    let x = required_index(tuple, 2, "value")?;
    ensure_numeric(dataset, &[x], "density value")?;
    let color = optional_index(tuple, 3)?;
    let x_key = &column(dataset, x)?.key;
    let mut transform = json!({"density": x_key, "as": ["__value", "__density"]});
    if let Some(color) = color {
        transform["groupby"] = json!([column(dataset, color)?.key]);
    }
    let mut encoding = json!({
        "x": {"field": "__value", "type": "quantitative", "title": x_key},
        "y": {"field": "__density", "type": "quantitative", "title": "Density"}
    });
    if let Some(color) = color {
        encoding.insert("color".to_owned(), field_def(dataset, color, false)?);
    }
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "transform": [transform],
        "mark": {"type": "area", "opacity": 0.45, "line": true, "tooltip": true},
        "encoding": encoding
    }))
}

fn dot_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 3, 5, "dot")?;
    let x = required_index(tuple, 2, "x")?;
    let y = optional_index(tuple, 3)?;
    let mut encoding = json!({"x": field_def(dataset, x, true)?});
    if let Some(y) = y {
        encoding.insert("y".to_owned(), field_def(dataset, y, false)?);
    } else {
        encoding.insert("y".to_owned(), json!({"value": 0}));
    }
    let mark = if option_string(options, "mode") == Some("rug") {
        json!({"type": "tick", "tooltip": true})
    } else {
        json!({"type": "point", "filled": true, "opacity": 0.7, "tooltip": true})
    };
    let mut spec = base_spec(dataset, mark, encoding);
    if option_bool(options, "jitter", false) || option_string(options, "mode") == Some("strip") {
        spec["transform"] = json!([
            {"window": [{"op": "row_number", "as": "__row"}]},
            {"calculate": "((datum.__row % 9) - 4) * 2", "as": "__jitter"}
        ]);
        spec["encoding"]["yOffset"] = json!({"field": "__jitter", "type": "quantitative"});
    }
    Ok(spec)
}

fn boxplot_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 4, 5, "boxplot")?;
    let category = required_index(tuple, 2, "category")?;
    let value = required_index(tuple, 3, "value")?;
    ensure_numeric(dataset, &[value], "boxplot value")?;
    Ok(base_spec(
        dataset,
        json!({"type": "boxplot", "extent": 1.5}),
        json!({"x": field_def(dataset, category, false)?, "y": field_def(dataset, value, true)?}),
    ))
}

fn boxplot_summary_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 8, 9, "precomputed boxplot")?;
    let category = required_index(tuple, 2, "category")?;
    let min = required_index(tuple, 3, "min")?;
    let q1 = required_index(tuple, 4, "q1")?;
    let median = required_index(tuple, 5, "median")?;
    let q3 = required_index(tuple, 6, "q3")?;
    let max = required_index(tuple, 7, "max")?;
    ensure_numeric(dataset, &[min, q1, median, q3, max], "box summary")?;
    let category_def = field_def(dataset, category, false)?;
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "layer": [
            {"mark": "rule", "encoding": {"x": category_def, "y": field_def(dataset, min, true)?, "y2": {"field": column(dataset, max)?.key}}},
            {"mark": {"type": "bar", "size": 24}, "encoding": {"x": field_def(dataset, category, false)?, "y": field_def(dataset, q1, true)?, "y2": {"field": column(dataset, q3)?.key}}},
            {"mark": {"type": "tick", "color": "white", "size": 24}, "encoding": {"x": field_def(dataset, category, false)?, "y": field_def(dataset, median, true)?}}
        ]
    }))
}

fn qq_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 3, 5, "QQ plot")?;
    let x = required_index(tuple, 2, "x")?;
    ensure_numeric(dataset, &[x], "QQ x")?;
    if let Some(y) = optional_index(tuple, 3)? {
        ensure_numeric(dataset, &[y], "QQ y")?;
        return Ok(base_spec(
            dataset,
            json!({"type": "point", "filled": true, "tooltip": true}),
            json!({"x": field_def(dataset, x, true)?, "y": field_def(dataset, y, true)?}),
        ));
    }
    let x_key = &column(dataset, x)?.key;
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "transform": [{"quantile": x_key, "probs": [0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99]}],
        "mark": {"type": "point", "filled": true, "tooltip": true},
        "encoding": {
            "x": {"field": "prob", "type": "quantitative", "title": "Probability"},
            "y": {"field": "value", "type": "quantitative", "title": x_key}
        }
    }))
}

fn scatter_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 4, 7, "scatter")?;
    let x = required_index(tuple, 2, "x")?;
    let y = required_index(tuple, 3, "y")?;
    ensure_numeric(dataset, &[x, y], "scatter x/y")?;
    let color = optional_index(tuple, 4)?;
    let size = optional_index(tuple, 5)?;
    if let Some(size) = size {
        ensure_numeric(dataset, &[size], "bubble size")?;
    }
    let mut encoding =
        json!({"x": field_def(dataset, x, true)?, "y": field_def(dataset, y, true)?});
    if let Some(color) = color {
        encoding.insert("color".to_owned(), field_def(dataset, color, false)?);
    }
    if let Some(size) = size {
        encoding.insert("size".to_owned(), field_def(dataset, size, true)?);
    }
    let shape = option_string(options, "shape").unwrap_or("circle");
    Ok(base_spec(
        dataset,
        json!({"type": "point", "shape": shape, "filled": true, "opacity": 0.75, "tooltip": true}),
        encoding,
    ))
}

fn parallel_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 3, 5, "parallel coordinates")?;
    let dimensions = required_indexes(tuple, 2, "dimensions")?;
    if dimensions.len() < 2 {
        return Err("parallel coordinates requires at least two dimensions".to_owned());
    }
    ensure_numeric(dataset, &dimensions, "parallel dimensions")?;
    let color = optional_index(tuple, 3)?;
    let fields = dimensions
        .iter()
        .map(|index| column(dataset, *index).map(|item| item.key.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut encoding = json!({
        "x": {"field": "__dimension", "type": "nominal", "title": "Dimension"},
        "y": {"field": "__value", "type": "quantitative", "title": "Value"},
        "detail": {"field": "__row", "type": "nominal"}
    });
    if let Some(color) = color {
        encoding.insert("color".to_owned(), field_def(dataset, color, false)?);
    }
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "transform": [
            {"window": [{"op": "row_number", "as": "__row"}]},
            {"fold": fields, "as": ["__dimension", "__value"]}
        ],
        "mark": {"type": "line", "opacity": 0.4},
        "encoding": encoding
    }))
}

fn ternary_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 7, "ternary")?;
    let a = required_index(tuple, 2, "a")?;
    let b = required_index(tuple, 3, "b")?;
    let c = required_index(tuple, 4, "c")?;
    ensure_numeric(dataset, &[a, b, c], "ternary values")?;
    let label = optional_index(tuple, 5)?;
    let a_key = &column(dataset, a)?.key;
    let b_key = &column(dataset, b)?.key;
    let c_key = &column(dataset, c)?.key;
    let total = format!(
        "({}+{}+{})",
        datum_field(a_key),
        datum_field(b_key),
        datum_field(c_key)
    );
    let mut encoding = json!({
        "x": {"field": "__x", "type": "quantitative", "axis": null},
        "y": {"field": "__y", "type": "quantitative", "axis": null}
    });
    if let Some(label) = label {
        encoding.insert(
            "tooltip".to_owned(),
            json!([field_def(dataset, label, false)?]),
        );
    }
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "transform": [
            {"calculate": format!("0.5 * (2 * {} + {}) / {}", datum_field(b_key), datum_field(c_key), total), "as": "__x"},
            {"calculate": format!("0.8660254 * {} / {}", datum_field(c_key), total), "as": "__y"}
        ],
        "mark": {"type": "point", "filled": true, "size": 80, "tooltip": true},
        "encoding": encoding
    }))
}

fn heatmap_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 4, 6, "heatmap")?;
    let x = required_index(tuple, 2, "x")?;
    let y = required_index(tuple, 3, "y")?;
    let value = optional_index(tuple, 4)?;
    if let Some(value) = value {
        ensure_numeric(dataset, &[value], "heatmap value")?;
    }
    let bins = option_bins(options);
    let mut x_def = field_def(dataset, x, false)?;
    let mut y_def = field_def(dataset, y, false)?;
    if let Some(bins) = bins {
        x_def["bin"] = json!({"maxbins": bins[0]});
        y_def["bin"] = json!({"maxbins": *bins.get(1).unwrap_or(&bins[0])});
        x_def["type"] = json!("quantitative");
        y_def["type"] = json!("quantitative");
    }
    let color = if let Some(value) = value {
        let mut definition = field_def(dataset, value, true)?;
        if let Some(aggregate) = option_string(options, "ag") {
            definition["aggregate"] = json!(aggregate);
        }
        definition
    } else {
        json!({"aggregate": "count", "type": "quantitative", "title": "Count"})
    };
    Ok(base_spec(
        dataset,
        json!({"type": "rect", "tooltip": true}),
        json!({"x": x_def, "y": y_def, "color": color}),
    ))
}

fn mosaic_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 6, "mosaic")?;
    let x = required_index(tuple, 2, "x")?;
    let y = required_index(tuple, 3, "y")?;
    let value = required_index(tuple, 4, "value")?;
    ensure_numeric(dataset, &[value], "mosaic value")?;
    Ok(base_spec(
        dataset,
        json!({"type": "rect", "tooltip": true}),
        json!({
            "x": field_def(dataset, x, false)?,
            "y": field_def(dataset, y, false)?,
            "color": field_def(dataset, value, true)?,
            "size": field_def(dataset, value, true)?
        }),
    ))
}

fn arc_spec(
    code: &str,
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 4, 5, "arc")?;
    let category = required_index(tuple, 2, "category")?;
    let value = required_index(tuple, 3, "value")?;
    ensure_numeric(dataset, &[value], "arc value")?;
    let mark = match code {
        "don" => json!({"type": "arc", "innerRadius": 70, "tooltip": true}),
        "rad" => json!({"type": "arc", "innerRadius": 18, "stroke": "white", "tooltip": true}),
        _ => json!({"type": "arc", "tooltip": true}),
    };
    let mut encoding = json!({
        "theta": field_def(dataset, value, true)?,
        "color": field_def(dataset, category, false)?
    });
    if code == "rad" {
        encoding.insert("radius".to_owned(), field_def(dataset, value, true)?);
    }
    Ok(base_spec(dataset, mark, encoding))
}

fn error_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
    band: bool,
) -> Result<Value, String> {
    expect_len(tuple, 5, 7, if band { "error band" } else { "error bar" })?;
    let x = required_index(tuple, 2, "x")?;
    let lower = required_index(tuple, 3, "lower")?;
    let upper = required_index(tuple, 4, "upper")?;
    ensure_numeric(dataset, &[lower, upper], "error bounds")?;
    let center = optional_index(tuple, 5)?;
    if let Some(center) = center {
        ensure_numeric(dataset, &[center], "error center")?;
    }
    if band {
        let mut layers = vec![json!({
            "mark": {"type": "area", "opacity": 0.25},
            "encoding": {"x": field_def(dataset, x, false)?, "y": field_def(dataset, lower, true)?, "y2": {"field": column(dataset, upper)?.key}}
        })];
        if let Some(center) = center {
            layers.push(json!({"mark": {"type": "line", "strokeWidth": 2}, "encoding": {"x": field_def(dataset, x, false)?, "y": field_def(dataset, center, true)?}}));
        }
        return Ok(
            json!({"$schema": VEGA_LITE_SCHEMA, "data": {"name": dataset.id}, "layer": layers}),
        );
    }
    let mut layers = vec![json!({
        "mark": "rule",
        "encoding": {"x": field_def(dataset, x, false)?, "y": field_def(dataset, lower, true)?, "y2": {"field": column(dataset, upper)?.key}}
    })];
    if let Some(center) = center {
        layers.push(json!({"mark": {"type": "tick", "size": 18}, "encoding": {"x": field_def(dataset, x, false)?, "y": field_def(dataset, center, true)?}}));
    }
    Ok(json!({"$schema": VEGA_LITE_SCHEMA, "data": {"name": dataset.id}, "layer": layers}))
}

fn candlestick_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 7, 9, "candlestick")?;
    let x = required_index(tuple, 2, "x")?;
    let open = required_index(tuple, 3, "open")?;
    let high = required_index(tuple, 4, "high")?;
    let low = required_index(tuple, 5, "low")?;
    let close = required_index(tuple, 6, "close")?;
    ensure_numeric(dataset, &[open, high, low, close], "candlestick values")?;
    let open_key = &column(dataset, open)?.key;
    let close_key = &column(dataset, close)?.key;
    Ok(json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "layer": [
            {"mark": "rule", "encoding": {"x": field_def(dataset, x, false)?, "y": field_def(dataset, low, true)?, "y2": {"field": column(dataset, high)?.key}}},
            {"mark": {"type": "bar", "size": 8, "tooltip": true}, "encoding": {
                "x": field_def(dataset, x, false)?, "y": field_def(dataset, open, true)?, "y2": {"field": close_key},
                "color": {"condition": {"test": format!("{} < {}", datum_field(open_key), datum_field(close_key)), "value": "#15803d"}, "value": "#b91c1c"}
            }}
        ]
    }))
}

fn text_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 5, 6, "text plot")?;
    let x = required_index(tuple, 2, "x")?;
    let y = required_index(tuple, 3, "y")?;
    let text = required_index(tuple, 4, "text")?;
    Ok(base_spec(
        dataset,
        json!({"type": "text", "tooltip": true}),
        json!({"x": field_def(dataset, x, false)?, "y": field_def(dataset, y, false)?, "text": field_def(dataset, text, false)?}),
    ))
}

fn tick_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 3, 5, "tick plot")?;
    let x = required_index(tuple, 2, "x")?;
    let y = optional_index(tuple, 3)?;
    let mut encoding = json!({"x": field_def(dataset, x, true)?});
    if let Some(y) = y {
        encoding.insert("y".to_owned(), field_def(dataset, y, false)?);
    }
    Ok(base_spec(
        dataset,
        json!({"type": "tick", "tooltip": true}),
        encoding,
    ))
}

fn rule_spec(
    tuple: &[Value],
    dataset: &DatasetMeta,
    _options: &Map<String, Value>,
) -> Result<Value, String> {
    expect_len(tuple, 6, 7, "rule plot")?;
    let x = required_index(tuple, 2, "x1")?;
    let x2 = required_index(tuple, 3, "x2")?;
    let y = required_index(tuple, 4, "y1")?;
    let y2 = required_index(tuple, 5, "y2")?;
    Ok(base_spec(
        dataset,
        json!({"type": "rule", "strokeWidth": 2, "tooltip": true}),
        json!({
            "x": field_def(dataset, x, false)?, "x2": {"field": column(dataset, x2)?.key},
            "y": field_def(dataset, y, false)?, "y2": {"field": column(dataset, y2)?.key}
        }),
    ))
}

fn multi_measure_encoding(
    dataset: &DatasetMeta,
    x: usize,
    ys: &[usize],
    options: &Map<String, Value>,
) -> Result<(Map<String, Value>, Vec<Value>), String> {
    if ys.is_empty() {
        return Err("chart requires at least one measure".to_owned());
    }
    let mut encoding = Map::new();
    encoding.insert("x".to_owned(), field_def(dataset, x, false)?);
    if ys.len() == 1 {
        let mut y = field_def(dataset, ys[0], true)?;
        if let Some(aggregate) = option_string(options, "ag") {
            y["aggregate"] = json!(aggregate);
        }
        encoding.insert("y".to_owned(), y);
        Ok((encoding, Vec::new()))
    } else {
        let fields = ys
            .iter()
            .map(|index| column(dataset, *index).map(|item| item.key.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        encoding.insert(
            "y".to_owned(),
            json!({"field": "__value", "type": "quantitative", "title": "Value"}),
        );
        encoding.insert(
            "color".to_owned(),
            json!({"field": "__series", "type": "nominal", "title": "Series"}),
        );
        Ok((
            encoding,
            vec![json!({"fold": fields, "as": ["__series", "__value"]})],
        ))
    }
}

fn base_spec(dataset: &DatasetMeta, mark: Value, encoding: Value) -> Value {
    json!({
        "$schema": VEGA_LITE_SCHEMA,
        "data": {"name": dataset.id},
        "mark": mark,
        "encoding": encoding
    })
}

fn with_common_options(mut spec: Value, options: &Map<String, Value>) -> Value {
    add_title(&mut spec, option_string(options, "t"));
    apply_common_encoding_options(&mut spec, options);
    apply_top_k(&mut spec, options.get("top").and_then(Value::as_u64));
    if option_bool(options, "lb", false) && spec.get("layer").is_none() {
        let original = spec.clone();
        let encoding = original
            .get("encoding")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let mut label_encoding = encoding.clone();
        if let Some(y) = encoding.get("y") {
            label_encoding["text"] = y.clone();
        }
        if let Some(object) = spec.as_object_mut() {
            object.remove("mark");
            object.remove("encoding");
            object.insert(
                    "layer".to_owned(),
                    json!([
                        {"mark": original.get("mark").cloned().unwrap_or_else(|| json!("point")), "encoding": encoding},
                        {"mark": {"type": "text", "dy": -8}, "encoding": label_encoding}
                    ]),
                );
        }
    }
    add_interaction(&mut spec, option_string(options, "sel"));
    spec
}

fn apply_common_encoding_options(spec: &mut Value, options: &Map<String, Value>) {
    if let Some(object) = spec.as_object_mut() {
        if let Some(encoding) = object.get_mut("encoding").and_then(Value::as_object_mut) {
            for (channel, definition) in encoding {
                let Some(definition) = definition.as_object_mut() else {
                    continue;
                };
                if channel == "x"
                    && let Some(sort) = option_string(options, "sort")
                    && sort != "none"
                {
                    definition.insert("sort".to_owned(), json!(sort));
                }
                if matches!(channel.as_str(), "x" | "y")
                    && definition.get("type").and_then(Value::as_str) == Some("quantitative")
                    && let Some(zero) = options.get("zero").and_then(Value::as_bool)
                {
                    let scale = definition
                        .entry("scale".to_owned())
                        .or_insert_with(|| json!({}));
                    if let Some(scale) = scale.as_object_mut() {
                        scale.insert("zero".to_owned(), json!(zero));
                    }
                }
                if channel == "color" && !option_bool(options, "lg", true) {
                    definition.insert("legend".to_owned(), Value::Null);
                }
            }
        }
        if let Some(mark) = object.get_mut("mark").and_then(Value::as_object_mut)
            && options.contains_key("tip")
        {
            mark.insert(
                "tooltip".to_owned(),
                json!(option_bool(options, "tip", true)),
            );
        }
        if let Some(resolve) = option_string(options, "resolve")
            && object.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "layer" | "facet" | "repeat" | "hconcat" | "vconcat"
                )
            })
        {
            object.insert(
                "resolve".to_owned(),
                json!({"scale": {"x": resolve, "y": resolve, "color": resolve}}),
            );
        }
        for key in ["layer", "hconcat", "vconcat"] {
            if let Some(children) = object.get_mut(key).and_then(Value::as_array_mut) {
                for child in children {
                    apply_common_encoding_options(child, options);
                }
            }
        }
        if let Some(child) = object.get_mut("spec") {
            apply_common_encoding_options(child, options);
        }
    }
}

fn apply_top_k(spec: &mut Value, top: Option<u64>) {
    let Some(top) = top else {
        return;
    };
    if spec.get("data").is_none() {
        return;
    }
    let Some(field) = first_quantitative_field(spec) else {
        return;
    };
    let transforms = spec.as_object_mut().and_then(|object| {
        object
            .entry("transform")
            .or_insert_with(|| json!([]))
            .as_array_mut()
    });
    if let Some(transforms) = transforms {
        transforms.push(json!({
            "window": [{"op": "rank", "as": "__top_rank"}],
            "sort": [{"field": field, "order": "descending"}]
        }));
        transforms.push(json!({"filter": format!("datum.__top_rank <= {top}")}));
    }
}

fn first_quantitative_field(value: &Value) -> Option<String> {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("quantitative")
                && let Some(field) = object.get("field").and_then(Value::as_str)
            {
                return Some(field.to_owned());
            }
            object.values().find_map(first_quantitative_field)
        }
        Value::Array(items) => items.iter().find_map(first_quantitative_field),
        _ => None,
    }
}

fn add_interaction(spec: &mut Value, selection: Option<&str>) {
    let Some(selection) = selection.filter(|item| *item != "none") else {
        return;
    };
    let color_field = first_color_field(spec).unwrap_or_else(|| "__series".to_owned());
    let (name, param, highlight) = match selection {
        "hover" => (
            "agent_hover",
            json!({"name": "agent_hover", "select": {"type": "point", "on": "pointerover", "clear": "pointerout", "nearest": true}}),
            true,
        ),
        "click" => (
            "agent_select",
            json!({"name": "agent_select", "select": {"type": "point", "on": "click", "clear": "dblclick"}}),
            true,
        ),
        "brush" => (
            "agent_brush",
            json!({"name": "agent_brush", "select": {"type": "interval"}}),
            true,
        ),
        "zoom" => (
            "agent_zoom",
            json!({"name": "agent_zoom", "select": {"type": "interval", "bind": "scales"}}),
            false,
        ),
        "legend" => (
            "agent_legend",
            json!({"name": "agent_legend", "select": {"type": "point", "fields": [color_field]}, "bind": "legend"}),
            true,
        ),
        _ => return,
    };
    if let Some(object) = spec.as_object_mut() {
        object.insert("params".to_owned(), json!([param]));
    }
    if highlight {
        add_selection_highlight(spec, name);
    }
}

fn add_selection_highlight(spec: &mut Value, param: &str) {
    if let Some(object) = spec.as_object_mut() {
        if let Some(encoding) = object.get_mut("encoding").and_then(Value::as_object_mut) {
            encoding.entry("opacity".to_owned()).or_insert_with(|| {
                json!({
                    "condition": {"param": param, "value": 1.0, "empty": true},
                    "value": 0.4
                })
            });
        }
        for key in ["layer", "hconcat", "vconcat"] {
            if let Some(children) = object.get_mut(key).and_then(Value::as_array_mut) {
                for child in children {
                    add_selection_highlight(child, param);
                }
            }
        }
        if let Some(child) = object.get_mut("spec") {
            add_selection_highlight(child, param);
        }
    }
}

fn first_color_field(value: &Value) -> Option<String> {
    match value {
        Value::Object(object) => {
            if let Some(field) = object
                .get("encoding")
                .and_then(|encoding| encoding.get("color"))
                .and_then(|color| color.get("field"))
                .and_then(Value::as_str)
            {
                return Some(field.to_owned());
            }
            object.values().find_map(first_color_field)
        }
        Value::Array(items) => items.iter().find_map(first_color_field),
        _ => None,
    }
}

fn add_title(spec: &mut Value, title: Option<&str>) {
    if let (Some(object), Some(title)) = (spec.as_object_mut(), title) {
        object.insert("title".to_owned(), json!(title));
    }
}

fn field_def(dataset: &DatasetMeta, index: usize, quantitative: bool) -> Result<Value, String> {
    let column = column(dataset, index)?;
    Ok(json!({
        "field": column.key,
        "type": if quantitative { "quantitative" } else { vega_type(&column.type_code) },
        "title": titleize(&column.key)
    }))
}

fn vega_type(type_code: &str) -> &'static str {
    match type_code {
        "n" | "cur" | "pct" => "quantitative",
        "d" | "dt" => "temporal",
        _ => "nominal",
    }
}

fn chart_name(code: &str) -> &'static str {
    match code {
        "ln" => "line",
        "ar" => "area",
        "tr" => "trail",
        "reg" => "regression",
        "bar" => "bar",
        "gantt" => "gantt",
        "water" => "waterfall",
        "bullet" => "bullet",
        "range" => "ranged-dot",
        "hist" => "histogram",
        "den" => "density",
        "dot" => "dot",
        "box" | "box5" => "boxplot",
        "qq" => "qq",
        "sc" => "scatter",
        "par" => "parallel-coordinates",
        "tri" => "ternary",
        "heat" => "heatmap",
        "mos" => "mosaic",
        "pie" => "pie",
        "don" => "donut",
        "rad" => "radial",
        "err" => "errorbar",
        "band" => "errorband",
        "candle" => "candlestick",
        "text" => "text",
        "tick" => "tick",
        "rule" => "rule",
        _ => "chart",
    }
}

fn options(tuple: &[Value]) -> Result<&Map<String, Value>, String> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    if let Some(object) = tuple.last().and_then(Value::as_object) {
        Ok(object)
    } else {
        Ok(EMPTY.get_or_init(Map::new))
    }
}

fn validate_options(options: &Map<String, Value>) -> Result<(), String> {
    for (key, value) in options {
        if !OPTION_KEYS.contains(&key.as_str()) {
            return Err(format!("unsupported chart option '{key}'"));
        }
        match key.as_str() {
            "t" if !value.is_string() => return Err("option 't' must be a string".to_owned()),
            "or" if !matches!(value.as_str(), Some("h" | "v")) => {
                return Err("option 'or' must be 'h' or 'v'".to_owned());
            }
            "st" if !matches!(
                value.as_str(),
                Some("none" | "zero" | "normalize" | "center")
            ) =>
            {
                return Err("option 'st' has an unsupported stack mode".to_owned());
            }
            "ag" if !matches!(
                value.as_str(),
                Some("sum" | "mean" | "median" | "min" | "max" | "count")
            ) =>
            {
                return Err("option 'ag' has an unsupported aggregate".to_owned());
            }
            "ip" if !matches!(
                value.as_str(),
                Some("linear" | "monotone" | "step" | "step-before" | "step-after")
            ) =>
            {
                return Err("option 'ip' has an unsupported interpolation".to_owned());
            }
            "mode" if !value.as_str().is_some_and(|item| MODES.contains(&item)) => {
                return Err("option 'mode' has an unsupported value".to_owned());
            }
            "pt" | "lb" | "lg" | "tip" | "zero" | "jitter" if !value.is_boolean() => {
                return Err(format!("option '{key}' must be a boolean"));
            }
            "sort" if !matches!(value.as_str(), Some("asc" | "desc" | "none")) => {
                return Err("option 'sort' has an unsupported value".to_owned());
            }
            "shape" if !matches!(value.as_str(), Some("circle" | "square" | "tick")) => {
                return Err("option 'shape' has an unsupported value".to_owned());
            }
            "sel"
                if !matches!(
                    value.as_str(),
                    Some("none" | "hover" | "click" | "brush" | "zoom" | "legend")
                ) =>
            {
                return Err("option 'sel' has an unsupported interaction".to_owned());
            }
            "resolve" if !matches!(value.as_str(), Some("shared" | "independent")) => {
                return Err("option 'resolve' has an unsupported value".to_owned());
            }
            "top" if value.as_u64().is_none_or(|item| item == 0) => {
                return Err("option 'top' must be a positive integer".to_owned());
            }
            "bn" if option_bins_value(value).is_none() => {
                return Err(
                    "option 'bn' must be a positive integer or two positive integers".to_owned(),
                );
            }
            _ => {}
        }
    }
    Ok(())
}

fn option_string<'a>(options: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    options.get(key).and_then(Value::as_str)
}

fn option_bool(options: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    options
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn option_bins(options: &Map<String, Value>) -> Option<Vec<u64>> {
    options.get("bn").and_then(option_bins_value)
}

fn option_bins_value(value: &Value) -> Option<Vec<u64>> {
    if let Some(value) = value.as_u64().filter(|value| *value > 0) {
        return Some(vec![value]);
    }
    let values = value.as_array()?;
    if values.len() != 2 {
        return None;
    }
    values
        .iter()
        .map(|item| item.as_u64().filter(|value| *value > 0))
        .collect()
}

fn expect_len(tuple: &[Value], min: usize, max: usize, label: &str) -> Result<(), String> {
    let actual = if tuple.last().is_some_and(Value::is_object) {
        tuple.len() - 1
    } else {
        tuple.len()
    };
    if (min..=max).contains(&actual) {
        Ok(())
    } else {
        Err(format!(
            "{label} tuple has {actual} positional entries; expected {min} to {max}"
        ))
    }
}

fn required_index(tuple: &[Value], position: usize, role: &str) -> Result<usize, String> {
    tuple
        .get(position)
        .and_then(Value::as_u64)
        .map(|item| item as usize)
        .ok_or_else(|| format!("{role} must be a non-negative column index"))
}

fn optional_index(tuple: &[Value], position: usize) -> Result<Option<usize>, String> {
    match tuple.get(position) {
        None | Some(Value::Null) | Some(Value::Object(_)) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(|item| Some(item as usize))
            .ok_or_else(|| format!("entry {position} must be a non-negative column index or null")),
    }
}

fn required_indexes(tuple: &[Value], position: usize, role: &str) -> Result<Vec<usize>, String> {
    let values = tuple
        .get(position)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{role} must be an array of column indexes"))?;
    if values.is_empty() {
        return Err(format!("{role} must not be empty"));
    }
    values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .map(|item| item as usize)
                .ok_or_else(|| format!("{role} entries must be non-negative column indexes"))
        })
        .collect()
}

fn optional_indexes(tuple: &[Value], position: usize) -> Result<Vec<usize>, String> {
    match tuple.get(position) {
        None | Some(Value::Null) | Some(Value::Object(_)) => Ok(Vec::new()),
        Some(_) => required_indexes(tuple, position, "column indexes"),
    }
}

fn column(dataset: &DatasetMeta, index: usize) -> Result<&ColumnMeta, String> {
    dataset.columns.get(index).ok_or_else(|| {
        format!(
            "column index {index} is out of range for dataset '{}'",
            dataset.id
        )
    })
}

fn ensure_numeric(dataset: &DatasetMeta, indexes: &[usize], role: &str) -> Result<(), String> {
    for index in indexes {
        let column = column(dataset, *index)?;
        if !matches!(column.type_code.as_str(), "n" | "cur" | "pct") {
            return Err(format!(
                "{role} column '{}' must be numeric-compatible",
                column.key
            ));
        }
    }
    Ok(())
}

fn datum_field(field: &str) -> String {
    format!(
        "datum[{}]",
        serde_json::to_string(field).unwrap_or_else(|_| "\"\"".to_owned())
    )
}

fn titleize(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                format!("{}{}", first.to_uppercase(), chars.as_str())
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn remove_schema(spec: &mut Value) {
    if let Some(object) = spec.as_object_mut() {
        object.remove("$schema");
    }
}

fn remove_data(spec: &mut Value) {
    if let Some(object) = spec.as_object_mut() {
        object.remove("data");
    }
}

fn deduplicate(values: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    values.retain(|item| seen.insert(item.clone()));
}

fn replace_first_quantitative_field(value: &mut Value, replacement: &Value) -> bool {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("quantitative")
                && object.contains_key("field")
            {
                object.insert("field".to_owned(), replacement.clone());
                return true;
            }
            object
                .values_mut()
                .any(|item| replace_first_quantitative_field(item, replacement))
        }
        Value::Array(items) => items
            .iter_mut()
            .any(|item| replace_first_quantitative_field(item, replacement)),
        _ => false,
    }
}
