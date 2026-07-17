use serde_json::Value;

use crate::domain;

pub const VERSION: u64 = 1;
pub const TOP_LEVEL_KEYS: &[&str] = &[
    "version", "t", "s", "theme", "density", "emphasis", "d", "m", "v", "a", "md", "dict", "i",
    "as",
];

pub const TYPE_CODE_STRING: &str = "s";
pub const TYPE_CODE_NUMBER: &str = "n";
pub const TYPE_CODE_CURRENCY: &str = "cur";
pub const TYPE_CODE_PERCENT: &str = "pct";
pub const TYPE_CODE_DATE: &str = "d";
pub const TYPE_CODE_DATETIME: &str = "dt";
pub const TYPE_CODE_BOOLEAN: &str = "b";
pub const TYPE_CODE_DICT_PREFIX: &str = "dict:";
pub const BASE_TYPE_CODES: &[&str] = &[
    TYPE_CODE_STRING,
    TYPE_CODE_NUMBER,
    TYPE_CODE_CURRENCY,
    TYPE_CODE_PERCENT,
    TYPE_CODE_DATE,
    TYPE_CODE_DATETIME,
    TYPE_CODE_BOOLEAN,
];
pub const DELTA_FORMAT_CODES: &[&str] = &[TYPE_CODE_NUMBER, TYPE_CODE_PERCENT];

pub const VIEW_CODE_OVERVIEW: &str = "o";
pub const VIEW_CODE_RECORDS: &str = "r";
pub const VIEW_CODE_TREND: &str = "t";
pub const VIEW_CODE_COMPARISON: &str = "b";
pub const VIEW_CODE_DISTRIBUTION: &str = "d";
pub const VIEW_CODE_COMPOSITION: &str = "p";
pub const VIEW_CODE_RELATIONSHIP: &str = "s";
pub const VIEW_CODES: &[&str] = &[
    VIEW_CODE_OVERVIEW,
    VIEW_CODE_RECORDS,
    VIEW_CODE_TREND,
    VIEW_CODE_COMPARISON,
    VIEW_CODE_DISTRIBUTION,
    VIEW_CODE_COMPOSITION,
    VIEW_CODE_RELATIONSHIP,
];
pub const SIMPLE_VIEW_CODES: &[&str] = &[VIEW_CODE_OVERVIEW, VIEW_CODE_RECORDS];
pub const MEASURE_VIEW_CODES: &[&str] = &[
    VIEW_CODE_TREND,
    VIEW_CODE_COMPARISON,
    VIEW_CODE_COMPOSITION,
    VIEW_CODE_RELATIONSHIP,
];

pub const ALERT_CODE_INFO: &str = "i";
pub const ALERT_CODE_SUCCESS: &str = "s";
pub const ALERT_CODE_WARNING: &str = "w";
pub const ALERT_CODE_ERROR: &str = "e";
pub const ALERT_CODE_CRITICAL: &str = "c";
pub const ALERT_LEVEL_CODES: &[&str] = &[
    ALERT_CODE_INFO,
    ALERT_CODE_SUCCESS,
    ALERT_CODE_WARNING,
    ALERT_CODE_ERROR,
    ALERT_CODE_CRITICAL,
];

#[must_use]
pub fn normalize_type_code(type_code: &str) -> String {
    if type_code.starts_with(TYPE_CODE_DICT_PREFIX) {
        return domain::COLUMN_TYPE_STRING.to_owned();
    }
    match type_code {
        TYPE_CODE_NUMBER => domain::COLUMN_TYPE_NUMBER,
        TYPE_CODE_CURRENCY => domain::COLUMN_TYPE_CURRENCY,
        TYPE_CODE_PERCENT => domain::COLUMN_TYPE_PERCENT,
        TYPE_CODE_DATE => domain::COLUMN_TYPE_DATE,
        TYPE_CODE_DATETIME => domain::COLUMN_TYPE_DATETIME,
        TYPE_CODE_BOOLEAN => domain::COLUMN_TYPE_BOOLEAN,
        _ => domain::COLUMN_TYPE_STRING,
    }
    .to_owned()
}

#[must_use]
pub fn normalize_metric_format(type_code: &str) -> String {
    match type_code {
        TYPE_CODE_NUMBER => domain::COLUMN_TYPE_NUMBER,
        TYPE_CODE_CURRENCY => domain::COLUMN_TYPE_CURRENCY,
        TYPE_CODE_PERCENT => domain::COLUMN_TYPE_PERCENT,
        _ => domain::COLUMN_TYPE_STRING,
    }
    .to_owned()
}

#[must_use]
pub fn view_intent_for_code(code: &str) -> Option<&'static str> {
    match code {
        VIEW_CODE_OVERVIEW => Some(domain::VIEW_INTENT_OVERVIEW),
        VIEW_CODE_RECORDS => Some(domain::VIEW_INTENT_PRECISE_RECORDS),
        VIEW_CODE_TREND => Some(domain::VIEW_INTENT_TREND),
        VIEW_CODE_COMPARISON => Some(domain::VIEW_INTENT_COMPARISON),
        VIEW_CODE_DISTRIBUTION => Some(domain::VIEW_INTENT_DISTRIBUTION),
        VIEW_CODE_COMPOSITION => Some(domain::VIEW_INTENT_COMPOSITION),
        VIEW_CODE_RELATIONSHIP => Some(domain::VIEW_INTENT_RELATIONSHIP),
        _ => None,
    }
}

#[must_use]
pub fn normalize_alert_level(code: &str) -> Option<&'static str> {
    match code {
        ALERT_CODE_INFO => Some(domain::ALERT_LEVEL_INFO),
        ALERT_CODE_SUCCESS => Some(domain::ALERT_LEVEL_SUCCESS),
        ALERT_CODE_WARNING => Some(domain::ALERT_LEVEL_WARNING),
        ALERT_CODE_ERROR => Some(domain::ALERT_LEVEL_ERROR),
        ALERT_CODE_CRITICAL => Some(domain::ALERT_LEVEL_CRITICAL),
        _ => None,
    }
}

#[must_use]
pub fn is_view_code(value: &str) -> bool {
    VIEW_CODES.contains(&value)
}

#[must_use]
pub fn is_simple_view_code(value: &str) -> bool {
    SIMPLE_VIEW_CODES.contains(&value)
}

#[must_use]
pub fn is_measure_view_code(value: &str) -> bool {
    MEASURE_VIEW_CODES.contains(&value)
}

#[must_use]
pub fn is_alert_level_code(value: &str) -> bool {
    ALERT_LEVEL_CODES.contains(&value)
}

#[must_use]
pub fn is_base_or_dict_type_code(value: &str) -> bool {
    BASE_TYPE_CODES.contains(&value) || value.starts_with(TYPE_CODE_DICT_PREFIX)
}

pub(super) fn is_primitive(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
}
