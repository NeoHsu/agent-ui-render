mod marks;
mod statistical;
mod temporal;

use marks::{arc_spec, candlestick_spec, error_spec, rule_spec, text_spec, tick_spec};
use statistical::{
    boxplot_spec, boxplot_summary_spec, density_spec, dot_spec, heatmap_spec, histogram_spec,
    mosaic_spec, parallel_spec, qq_spec, scatter_spec, ternary_spec,
};
use temporal::{
    bar_spec, bullet_spec, gantt_spec, line_or_area_spec, range_spec, regression_spec, trail_spec,
    waterfall_spec,
};

use serde_json::{Map, Value, json};

use super::{
    DatasetMeta, VEGA_LITE_SCHEMA, column, field_def, option_string, series_legend_label_expr,
    with_common_options,
};

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

pub(super) fn build_chart_spec(
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
            json!({
                "field": "__series",
                "type": "nominal",
                "title": "Series",
                "legend": {"labelExpr": series_legend_label_expr(&fields)}
            }),
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
