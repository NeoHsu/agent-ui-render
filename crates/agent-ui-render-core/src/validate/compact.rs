mod content;
mod datasets;
mod views;

use serde_json::Value;

use crate::{diagnostic::ValidationReport, options::Limits, wire::compact};

use super::{
    shared::{
        validate_presentation_options, validate_string_array, validate_string_length,
        validate_unknown_fields,
    },
    unsafe_content::collect_unsafe_string_paths,
};
use content::{validate_compact_alerts, validate_compact_markdown, validate_compact_metrics};
use datasets::{validate_compact_datasets, validate_dictionaries};
use views::validate_compact_views;

#[must_use]
pub(super) fn validate_compact_report(value: &Value, limits: &Limits) -> ValidationReport {
    let mut report = ValidationReport::default();
    let Some(object) = value.as_object() else {
        report.error("$", "top-level value must be an object");
        return report;
    };

    validate_unknown_fields("$", object, compact::TOP_LEVEL_KEYS, &mut report);

    let version = object.get("version").and_then(Value::as_u64);
    if !matches!(version, Some(compact::VERSION | crate::wire::v2::VERSION)) {
        report.error("$.version", "field 'version' must be exactly 1 or 2");
    }

    for key in ["t", "s"] {
        if let Some(item) = object.get(key) {
            if let Some(text) = item.as_str() {
                validate_string_length(
                    text,
                    &format!("$.{key}"),
                    limits.max_string_chars,
                    &format!("field '{key}'"),
                    &mut report,
                );
            } else {
                report.error(
                    format!("$.{key}"),
                    format!("field '{key}' must be a string when present"),
                );
            }
        }
    }
    validate_presentation_options(object, &mut report);

    let dictionaries = validate_dictionaries(object.get("dict"), limits, &mut report);
    let datasets = validate_compact_datasets(object.get("d"), &dictionaries, limits, &mut report);

    validate_compact_metrics(object.get("m"), limits, &mut report);
    validate_string_array("i", object.get("i"), limits, &mut report);
    validate_string_array("as", object.get("as"), limits, &mut report);
    validate_compact_views(
        object.get("v"),
        version.unwrap_or(compact::VERSION),
        &datasets,
        limits,
        &mut report,
    );
    validate_compact_alerts(object.get("a"), limits, &mut report);
    validate_compact_markdown(object.get("md"), limits, &mut report);

    for path in collect_unsafe_string_paths(value) {
        report.error(
            path.clone(),
            format!("unsafe UI/code content detected at {path}"),
        );
    }

    let has_datasets = object
        .get("d")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let views_len = object
        .get("v")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let metrics_len = object
        .get("m")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let markdown_len = object
        .get("md")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if has_datasets
        && views_len == 0
        && metrics_len == 0
        && markdown_len == 0
        && object.get("s").is_none()
    {
        report.warning(
            "$",
            "compact payload has datasets but no views, metrics, markdown, or summary; renderer will likely fall back to a table",
        );
    }

    report
}
