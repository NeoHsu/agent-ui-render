use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const NORMALIZED_SCHEMA: &str = "ui.input.normalized";
pub const SPEC_SCHEMA: &str = "ui.spec";
pub const FORMAT_VERSION: u32 = 1;

pub const THEME_REPORT_LIGHT: &str = "report-light";
pub const THEME_TECHNICAL_DARK: &str = "technical-dark";
pub const THEME_EXECUTIVE_CLEAN: &str = "executive-clean";
pub const THEMES: &[&str] = &[
    THEME_REPORT_LIGHT,
    THEME_TECHNICAL_DARK,
    THEME_EXECUTIVE_CLEAN,
];

pub const DENSITY_COMFORTABLE: &str = "comfortable";
pub const DENSITY_COMPACT: &str = "compact";
pub const DENSITIES: &[&str] = &[DENSITY_COMFORTABLE, DENSITY_COMPACT];

pub const EMPHASIS_STRONG: &str = "strong";
pub const EMPHASIS_SUBTLE: &str = "subtle";
pub const EMPHASES: &[&str] = &[EMPHASIS_STRONG, EMPHASIS_SUBTLE];

pub const COLUMN_TYPE_STRING: &str = "string";
pub const COLUMN_TYPE_NUMBER: &str = "number";
pub const COLUMN_TYPE_CURRENCY: &str = "currency";
pub const COLUMN_TYPE_PERCENT: &str = "percent";
pub const COLUMN_TYPE_DATE: &str = "date";
pub const COLUMN_TYPE_DATETIME: &str = "datetime";
pub const COLUMN_TYPE_BOOLEAN: &str = "boolean";
pub const COLUMN_TYPES: &[&str] = &[
    COLUMN_TYPE_STRING,
    COLUMN_TYPE_NUMBER,
    COLUMN_TYPE_CURRENCY,
    COLUMN_TYPE_PERCENT,
    COLUMN_TYPE_DATE,
    COLUMN_TYPE_DATETIME,
    COLUMN_TYPE_BOOLEAN,
];

pub const METRIC_FORMATS: &[&str] = &[
    COLUMN_TYPE_NUMBER,
    COLUMN_TYPE_CURRENCY,
    COLUMN_TYPE_PERCENT,
    COLUMN_TYPE_STRING,
];

pub const VIEW_INTENT_OVERVIEW: &str = "overview";
pub const VIEW_INTENT_PRECISE_RECORDS: &str = "precise_records";
pub const VIEW_INTENT_TREND: &str = "trend";
pub const VIEW_INTENT_COMPARISON: &str = "comparison";
pub const VIEW_INTENT_DISTRIBUTION: &str = "distribution";
pub const VIEW_INTENT_COMPOSITION: &str = "composition";
pub const VIEW_INTENT_RELATIONSHIP: &str = "relationship";
pub const VIEW_INTENTS: &[&str] = &[
    VIEW_INTENT_OVERVIEW,
    VIEW_INTENT_PRECISE_RECORDS,
    VIEW_INTENT_TREND,
    VIEW_INTENT_COMPARISON,
    VIEW_INTENT_DISTRIBUTION,
    VIEW_INTENT_COMPOSITION,
    VIEW_INTENT_RELATIONSHIP,
];

pub const ALERT_LEVEL_INFO: &str = "info";
pub const ALERT_LEVEL_SUCCESS: &str = "success";
pub const ALERT_LEVEL_WARNING: &str = "warning";
pub const ALERT_LEVEL_ERROR: &str = "error";
pub const ALERT_LEVEL_CRITICAL: &str = "critical";
pub const ALERT_LEVELS: &[&str] = &[
    ALERT_LEVEL_INFO,
    ALERT_LEVEL_SUCCESS,
    ALERT_LEVEL_WARNING,
    ALERT_LEVEL_ERROR,
    ALERT_LEVEL_CRITICAL,
];

pub type Primitive = Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub column_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dataset {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Primitive>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricDelta {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    pub label: String,
    pub value: Primitive,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<MetricDelta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewIntent {
    pub intent: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measures: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    pub level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema: String,
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emphasis: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub datasets: BTreeMap<String, Dataset>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub metrics: Vec<Metric>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub insights: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub markdown: Vec<MarkdownSection>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub views: Vec<ViewIntent>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alerts: Vec<Alert>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub assumptions: Vec<String>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            schema: NORMALIZED_SCHEMA.to_owned(),
            version: FORMAT_VERSION,
            title: None,
            summary: None,
            theme: None,
            density: None,
            emphasis: None,
            datasets: BTreeMap::new(),
            metrics: Vec::new(),
            insights: Vec::new(),
            markdown: Vec::new(),
            views: Vec::new(),
            alerts: Vec::new(),
            assumptions: Vec::new(),
        }
    }
}
