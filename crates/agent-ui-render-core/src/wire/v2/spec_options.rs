use serde_json::{Map, Value, json};

use super::options::{option_bool, option_string};

pub(super) fn with_common_options(mut spec: Value, options: &Map<String, Value>) -> Value {
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

pub(super) fn add_interaction(spec: &mut Value, selection: Option<&str>) {
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
    insert_interaction_param(spec, param);
    if highlight {
        add_selection_highlight(spec, name);
    }
}

// Selection params must live in a unit spec: placing them on a layered spec
// makes the Vega-Lite compiler clone them into every layer, which produces
// duplicate runtime signal names and aborts rendering.
fn insert_interaction_param(spec: &mut Value, param: Value) {
    let Some(object) = spec.as_object_mut() else {
        return;
    };
    if let Some(first) = object
        .get_mut("layer")
        .and_then(Value::as_array_mut)
        .and_then(|layers| layers.first_mut())
    {
        insert_interaction_param(first, param);
        return;
    }
    if let Some(child) = object.get_mut("spec") {
        insert_interaction_param(child, param);
        return;
    }
    match object.get_mut("params").and_then(Value::as_array_mut) {
        Some(params) => params.push(param),
        None => {
            object.insert("params".to_owned(), json!([param]));
        }
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

pub(super) fn add_title(spec: &mut Value, title: Option<&str>) {
    if let (Some(object), Some(title)) = (spec.as_object_mut(), title) {
        object.insert("title".to_owned(), json!(title));
    }
}
