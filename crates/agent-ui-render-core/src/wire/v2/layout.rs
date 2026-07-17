use serde_json::{Map, Value, json};

use super::{
    DatasetMeta, LAYOUT_CODES, VEGA_LITE_SCHEMA, add_interaction, add_title, chart_view, column,
    deduplicate, expect_len, field_def, normalize_view, option_string, optional_index, options,
    remove_data, remove_schema, replace_first_quantitative_field, validate_options,
};

pub(super) fn normalize_layout(
    tuple: &[Value],
    index: usize,
    datasets: &[DatasetMeta],
) -> Result<crate::domain::ViewIntent, String> {
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
            add_interaction(&mut spec, option_string(options, "sel"));
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
