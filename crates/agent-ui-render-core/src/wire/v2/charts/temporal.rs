use serde_json::{Map, Value, json};

use super::super::{
    DatasetMeta, VEGA_LITE_SCHEMA, column, datum_field, ensure_numeric, expect_len, field_def,
    option_bool, option_string, optional_index, optional_indexes, required_index, required_indexes,
};
use super::{InsertProperty, base_spec, multi_measure_encoding};

pub(super) fn line_or_area_spec(
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

pub(super) fn trail_spec(
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
    let mut size = field_def(dataset, width, true)?;
    size["legend"] = json!({"tickCount": 3});
    let mut encoding = json!({
        "x": field_def(dataset, x, false)?,
        "y": field_def(dataset, y, true)?,
        "size": size
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

pub(super) fn regression_spec(
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

pub(super) fn bar_spec(
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

pub(super) fn gantt_spec(
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

pub(super) fn waterfall_spec(
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

pub(super) fn bullet_spec(
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

pub(super) fn range_spec(
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
