use serde_json::{Map, Value, json};

use crate::{
    chart::{chart_kind_for_view, first_non_measure_column, first_numeric_columns},
    domain::{Dataset, FORMAT_VERSION, Report, SPEC_SCHEMA, ViewIntent},
};

#[must_use]
pub fn plan_ui_spec(input: &Report) -> Value {
    let mut spec = Map::new();
    spec.insert("schema".to_owned(), json!(SPEC_SCHEMA));
    spec.insert("version".to_owned(), json!(FORMAT_VERSION));
    if let Some(title) = &input.title {
        spec.insert("title".to_owned(), json!(title));
    }
    if let Some(summary) = &input.summary {
        spec.insert("summary".to_owned(), json!(summary));
    }
    if let Some(theme) = &input.theme {
        spec.insert("theme".to_owned(), json!(theme));
    }
    if let Some(density) = &input.density {
        spec.insert("density".to_owned(), json!(density));
    }
    if let Some(emphasis) = &input.emphasis {
        spec.insert("emphasis".to_owned(), json!(emphasis));
    }
    if !input.datasets.is_empty() {
        spec.insert("datasets".to_owned(), json!(input.datasets));
    }

    let mut blocks = Vec::new();
    add_metric_blocks(&mut blocks, input);
    add_insight_blocks(&mut blocks, input);
    add_markdown_blocks(&mut blocks, input);
    add_view_blocks(&mut blocks, input);
    add_alert_blocks(&mut blocks, input);
    add_assumption_blocks(&mut blocks, input);
    spec.insert("blocks".to_owned(), Value::Array(blocks));
    Value::Object(spec)
}

fn add_metric_blocks(blocks: &mut Vec<Value>, input: &Report) {
    for (index, metric) in input.metrics.iter().enumerate() {
        let mut block = Map::new();
        block.insert(
            "id".to_owned(),
            json!(format!("metric_{}", slug(&metric.label, index + 1))),
        );
        block.insert("type".to_owned(), json!("metric"));
        block.insert("label".to_owned(), json!(metric.label));
        block.insert("value".to_owned(), metric.value.clone());
        if let Some(format) = &metric.format {
            block.insert("format".to_owned(), json!(format));
        }
        if let Some(unit) = &metric.unit {
            block.insert("unit".to_owned(), json!(unit));
        }
        if let Some(delta) = &metric.delta {
            block.insert("delta".to_owned(), json!(delta));
        }
        blocks.push(Value::Object(block));
    }
}

fn add_insight_blocks(blocks: &mut Vec<Value>, input: &Report) {
    for (index, insight) in input.insights.iter().enumerate() {
        blocks.push(json!({
            "id": format!("insight_{}", index + 1),
            "type": "text",
            "variant": "insight",
            "content": insight,
        }));
    }
}

fn add_markdown_blocks(blocks: &mut Vec<Value>, input: &Report) {
    for (index, section) in input.markdown.iter().enumerate() {
        let mut block = Map::new();
        block.insert("id".to_owned(), json!(format!("markdown_{}", index + 1)));
        block.insert("type".to_owned(), json!("markdown"));
        block.insert("content".to_owned(), json!(section.content));
        if let Some(title) = &section.title {
            block.insert("title".to_owned(), json!(title));
        }
        blocks.push(Value::Object(block));
    }
}

fn add_view_blocks(blocks: &mut Vec<Value>, input: &Report) {
    for (index, view) in input.views.iter().enumerate() {
        let Some(dataset) = input.datasets.get(&view.data) else {
            continue;
        };
        if view.intent == "overview" {
            if let Some(title) = &view.title {
                blocks.push(json!({
                    "id": format!("overview_{}", index + 1),
                    "type": "text",
                    "variant": "paragraph",
                    "content": title,
                }));
            }
            continue;
        }
        if view.intent == "precise_records" {
            let mut block = Map::new();
            block.insert(
                "id".to_owned(),
                json!(format!("table_{}", slug(&view.data, index + 1))),
            );
            block.insert("type".to_owned(), json!("table"));
            block.insert("data".to_owned(), json!(view.data));
            let columns = selected_table_column_keys(dataset, view);
            if !columns.is_empty() {
                block.insert("columns".to_owned(), json!(columns));
            }
            if let Some(title) = &view.title {
                block.insert("title".to_owned(), json!(title));
            }
            blocks.push(Value::Object(block));
            continue;
        }

        let measures = view.measures.clone().unwrap_or_default();
        let x = view
            .x
            .clone()
            .or_else(|| first_non_measure_column(dataset, &measures));
        let y = if measures.is_empty() {
            first_numeric_columns(dataset, 1)
        } else {
            measures
        };
        if x.is_none() || y.is_empty() {
            blocks.push(json!({
                "id": format!("table_fallback_{}", slug(&view.data, index + 1)),
                "type": "table",
                "data": view.data,
                "title": view.title.clone().unwrap_or_else(|| format!("{} data", view.intent.replace('_', " "))),
            }));
            continue;
        }
        let mut block = Map::new();
        block.insert(
            "id".to_owned(),
            json!(format!(
                "chart_{}_{}_{}",
                slug(&view.data, index + 1),
                slug(&view.intent, index + 1),
                index + 1
            )),
        );
        block.insert("type".to_owned(), json!("chart"));
        block.insert(
            "chart".to_owned(),
            json!(chart_kind_for_view(view, dataset)),
        );
        block.insert("data".to_owned(), json!(view.data));
        block.insert("x".to_owned(), json!(x.expect("checked")));
        block.insert("y".to_owned(), json!(y));
        if let Some(title) = &view.title {
            block.insert("title".to_owned(), json!(title));
        }
        blocks.push(Value::Object(block));
    }
}

fn add_alert_blocks(blocks: &mut Vec<Value>, input: &Report) {
    for (index, alert) in input.alerts.iter().enumerate() {
        let mut block = Map::new();
        block.insert("id".to_owned(), json!(format!("alert_{}", index + 1)));
        block.insert("type".to_owned(), json!("alert"));
        block.insert("level".to_owned(), json!(alert.level));
        block.insert("content".to_owned(), json!(alert.content));
        if let Some(title) = &alert.title {
            block.insert("title".to_owned(), json!(title));
        }
        blocks.push(Value::Object(block));
    }
}

fn add_assumption_blocks(blocks: &mut Vec<Value>, input: &Report) {
    for (index, assumption) in input.assumptions.iter().enumerate() {
        blocks.push(json!({
            "id": format!("assumption_{}", index + 1),
            "type": "text",
            "variant": "assumption",
            "content": assumption,
        }));
    }
}

fn selected_table_column_keys(dataset: &Dataset, view: &ViewIntent) -> Vec<String> {
    let requested = view
        .columns
        .as_ref()
        .filter(|columns| !columns.is_empty())
        .map_or_else(
            || {
                let mut fallback = Vec::new();
                if let Some(x) = &view.x {
                    fallback.push(x.as_str());
                }
                if let Some(dimensions) = &view.dimensions {
                    fallback.extend(dimensions.iter().map(String::as_str));
                }
                if let Some(measures) = &view.measures {
                    fallback.extend(measures.iter().map(String::as_str));
                }
                fallback
            },
            |columns| columns.iter().map(String::as_str).collect(),
        );

    let mut columns = Vec::new();
    for key in requested {
        if dataset.columns.iter().any(|column| column.key == key)
            && !columns.iter().any(|column| column == key)
        {
            columns.push(key.to_owned());
        }
    }
    columns
}

fn slug(value: &str, fallback: usize) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if slug.is_empty() {
        fallback.to_string()
    } else {
        slug
    }
}

#[allow(dead_code)]
fn _dataset_is_empty(dataset: &Dataset) -> bool {
    dataset.columns.is_empty() && dataset.rows.is_empty()
}
