use serde_json::{Map, Value, json};

use super::super::{
    DatasetMeta, VEGA_LITE_SCHEMA, column, datum_field, ensure_numeric, expect_len, field_def,
    option_bins, option_bool, option_string, optional_index, required_index, required_indexes,
    titleize,
};
use super::{InsertProperty, base_spec};

pub(super) fn histogram_spec(
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
            "x": {
                "field": column(dataset, x)?.key,
                "type": "quantitative",
                "bin": {"maxbins": maxbins},
                "title": titleize(&column(dataset, x)?.key)
            },
            "y": {"aggregate": "count", "type": "quantitative", "title": "Count"}
        }),
    ))
}

pub(super) fn density_spec(
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

pub(super) fn dot_spec(
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

pub(super) fn boxplot_spec(
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

pub(super) fn boxplot_summary_spec(
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

pub(super) fn qq_spec(
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

pub(super) fn scatter_spec(
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
        let mut size_definition = field_def(dataset, size, true)?;
        size_definition["legend"] = json!({"tickCount": 3});
        encoding.insert("size".to_owned(), size_definition);
    }
    let shape = option_string(options, "shape").unwrap_or("circle");
    Ok(base_spec(
        dataset,
        json!({"type": "point", "shape": shape, "filled": true, "opacity": 0.75, "tooltip": true}),
        encoding,
    ))
}

pub(super) fn parallel_spec(
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

pub(super) fn ternary_spec(
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

pub(super) fn heatmap_spec(
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
        json!({
            "aggregate": "count",
            "type": "quantitative",
            "title": "Count",
            "legend": {"format": "d"}
        })
    };
    Ok(base_spec(
        dataset,
        json!({"type": "rect", "tooltip": true}),
        json!({"x": x_def, "y": y_def, "color": color}),
    ))
}

pub(super) fn mosaic_spec(
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
            "size": {
                "field": column(dataset, value)?.key,
                "type": "quantitative",
                "legend": null
            }
        }),
    ))
}
