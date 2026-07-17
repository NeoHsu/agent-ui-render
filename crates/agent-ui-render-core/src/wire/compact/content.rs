use serde_json::Value;

use crate::domain::{self, Alert, MarkdownSection, Metric, MetricDelta};

use super::{
    DELTA_FORMAT_CODES, codes::is_primitive, normalize_alert_level, normalize_metric_format,
};

pub(super) fn normalize_compact_metrics(value: Option<&Value>) -> Vec<Metric> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|metric| {
            let tuple = metric.as_array()?;
            let label = tuple.first()?.as_str()?.to_owned();
            let value = tuple
                .get(1)
                .filter(|item| is_primitive(item))
                .cloned()
                .unwrap_or(Value::Null);
            let format = tuple
                .get(2)
                .and_then(Value::as_str)
                .map(normalize_metric_format);
            let unit = tuple.get(3).and_then(Value::as_str).map(ToOwned::to_owned);
            Some(Metric {
                label,
                value,
                format,
                unit,
                delta: normalize_compact_metric_delta(tuple.get(4)),
            })
        })
        .collect()
}

fn normalize_compact_metric_delta(value: Option<&Value>) -> Option<MetricDelta> {
    let value = value?;
    let (number, format_code) = if let Some(number) = value.as_f64() {
        (number, None)
    } else {
        let tuple = value.as_array()?;
        (
            tuple.first()?.as_f64()?,
            tuple.get(1).and_then(Value::as_str),
        )
    };
    let format = format_code
        .filter(|code| DELTA_FORMAT_CODES.contains(code))
        .map(normalize_metric_format);
    let direction = if number > 0.0 {
        domain::DELTA_DIRECTION_UP
    } else if number < 0.0 {
        domain::DELTA_DIRECTION_DOWN
    } else {
        domain::DELTA_DIRECTION_FLAT
    };
    Some(MetricDelta {
        label: Some(delta_label(number, format.as_deref())),
        value: number,
        format,
        direction: Some(direction.to_owned()),
    })
}

fn delta_label(value: f64, format: Option<&str>) -> String {
    let magnitude = if format == Some(domain::COLUMN_TYPE_PERCENT) {
        format!("{:.1}%", value.abs() * 100.0)
    } else if value.abs().fract() < f64::EPSILON {
        format!("{:.0}", value.abs())
    } else {
        format!("{:.2}", value.abs())
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    };
    if value > 0.0 {
        format!("+{magnitude}")
    } else if value < 0.0 {
        format!("-{magnitude}")
    } else {
        magnitude
    }
}

pub(super) fn normalize_compact_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn normalize_compact_alerts(value: Option<&Value>) -> Vec<Alert> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|alert| {
            let tuple = alert.as_array()?;
            let level = tuple
                .first()?
                .as_str()
                .and_then(normalize_alert_level)?
                .to_owned();
            match tuple.len() {
                2 => Some(Alert {
                    level,
                    title: None,
                    content: tuple.get(1)?.as_str()?.to_owned(),
                }),
                3.. => Some(Alert {
                    level,
                    title: Some(tuple.get(1)?.as_str()?.to_owned()),
                    content: tuple.get(2)?.as_str()?.to_owned(),
                }),
                _ => None,
            }
        })
        .collect()
}

pub(super) fn normalize_compact_markdown(value: Option<&Value>) -> Vec<MarkdownSection> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|section| {
            let tuple = section.as_array()?;
            match tuple.len() {
                1 => Some(MarkdownSection {
                    title: None,
                    content: tuple.first()?.as_str()?.to_owned(),
                }),
                2.. => Some(MarkdownSection {
                    title: Some(tuple.first()?.as_str()?.to_owned()),
                    content: tuple.get(1)?.as_str()?.to_owned(),
                }),
                _ => None,
            }
        })
        .collect()
}
