use serde_json::{Map, Value, json};

use super::super::{
    DatasetMeta, VEGA_LITE_SCHEMA, column, datum_field, ensure_numeric, expect_len, field_def,
    optional_index, required_index,
};
use super::{InsertProperty, base_spec};

pub(super) fn arc_spec(
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

pub(super) fn error_spec(
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

pub(super) fn candlestick_spec(
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

pub(super) fn text_spec(
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

pub(super) fn tick_spec(
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

pub(super) fn rule_spec(
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
