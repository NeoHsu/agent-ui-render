use std::sync::OnceLock;

use serde_json::{Map, Value};

const OPTION_KEYS: &[&str] = &[
    "t", "or", "st", "ag", "bn", "ip", "mode", "pt", "lb", "lg", "tip", "sort", "top", "zero",
    "shape", "jitter", "sel", "resolve",
];

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

pub(super) fn options(tuple: &[Value]) -> Result<&Map<String, Value>, String> {
    static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
    if let Some(object) = tuple.last().and_then(Value::as_object) {
        Ok(object)
    } else {
        Ok(EMPTY.get_or_init(Map::new))
    }
}

pub(super) fn validate_options(options: &Map<String, Value>) -> Result<(), String> {
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

pub(super) fn option_string<'a>(options: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    options.get(key).and_then(Value::as_str)
}

pub(super) fn option_bool(options: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    options
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

pub(super) fn option_bins(options: &Map<String, Value>) -> Option<Vec<u64>> {
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

pub(super) fn expect_len(
    tuple: &[Value],
    min: usize,
    max: usize,
    label: &str,
) -> Result<(), String> {
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

pub(super) fn required_index(
    tuple: &[Value],
    position: usize,
    role: &str,
) -> Result<usize, String> {
    tuple
        .get(position)
        .and_then(Value::as_u64)
        .map(|item| item as usize)
        .ok_or_else(|| format!("{role} must be a non-negative column index"))
}

pub(super) fn optional_index(tuple: &[Value], position: usize) -> Result<Option<usize>, String> {
    match tuple.get(position) {
        None | Some(Value::Null) | Some(Value::Object(_)) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(|item| Some(item as usize))
            .ok_or_else(|| format!("entry {position} must be a non-negative column index or null")),
    }
}

pub(super) fn required_indexes(
    tuple: &[Value],
    position: usize,
    role: &str,
) -> Result<Vec<usize>, String> {
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

pub(super) fn optional_indexes(tuple: &[Value], position: usize) -> Result<Vec<usize>, String> {
    match tuple.get(position) {
        None | Some(Value::Null) | Some(Value::Object(_)) => Ok(Vec::new()),
        Some(_) => required_indexes(tuple, position, "column indexes"),
    }
}
