mod content;
mod datasets;
mod views;

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{diagnostic::ValidationReport, domain, options::ValidationOptions};

use super::{
    shared::{
        DatasetInfo, validate_count, validate_dataset_totals, validate_presentation_options,
        validate_string_array, validate_string_length, validate_unknown_fields,
    },
    unsafe_content::collect_unsafe_string_paths,
};
use content::{
    validate_normalized_alerts, validate_normalized_markdown, validate_normalized_metrics,
};
use datasets::validate_normalized_dataset;
use views::validate_normalized_views;

const NORMALIZED_TOP_LEVEL_KEYS: &[&str] = &[
    "schema",
    "version",
    "title",
    "summary",
    "theme",
    "density",
    "emphasis",
    "datasets",
    "metrics",
    "insights",
    "markdown",
    "views",
    "alerts",
    "assumptions",
];

#[must_use]
pub(super) fn validate_normalized_report_with_options(
    value: &Value,
    options: &ValidationOptions,
) -> ValidationReport {
    let limits = &options.limits;
    let mut report = ValidationReport::with_max_findings(limits.max_findings);
    let Some(object) = value.as_object() else {
        report.error("$", "top-level value must be an object");
        return report;
    };

    validate_unknown_fields("$", object, NORMALIZED_TOP_LEVEL_KEYS, &mut report);

    if object.get("schema").and_then(Value::as_str) != Some(domain::NORMALIZED_SCHEMA) {
        report.error(
            "$.schema",
            "field 'schema' must be exactly 'ui.input.normalized'",
        );
    }
    let version = object.get("version").and_then(Value::as_u64);
    if !matches!(
        version,
        Some(version) if version == u64::from(domain::FORMAT_VERSION)
            || version == u64::from(domain::FORMAT_VERSION_V2)
    ) {
        report.error("$.version", "field 'version' must be exactly 1 or 2");
    }

    for key in ["title", "summary"] {
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

    let mut datasets_info: BTreeMap<String, DatasetInfo> = BTreeMap::new();
    let datasets = match object.get("datasets") {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => {
            report.error(
                "$.datasets",
                "field 'datasets' must be an object when present",
            );
            None
        }
    };
    if let Some(datasets) = datasets {
        validate_count(
            datasets.len(),
            limits.max_datasets,
            "$.datasets",
            "datasets",
            &mut report,
        );
        for (dataset_id, dataset) in datasets.iter().take(limits.max_datasets) {
            let path = format!("$.datasets.{dataset_id}");
            if let Some(info) =
                validate_normalized_dataset(dataset_id, dataset, &path, limits, &mut report)
            {
                datasets_info.insert(dataset_id.clone(), info);
            }
        }
    }
    validate_dataset_totals(datasets_info.values(), "$.datasets", limits, &mut report);

    validate_normalized_metrics(object.get("metrics"), limits, &mut report);
    validate_string_array("insights", object.get("insights"), limits, &mut report);
    validate_normalized_markdown(object.get("markdown"), limits, &mut report);
    validate_string_array(
        "assumptions",
        object.get("assumptions"),
        limits,
        &mut report,
    );
    validate_normalized_views(
        object.get("views"),
        version.unwrap_or(u64::from(domain::FORMAT_VERSION)),
        &datasets_info,
        limits,
        &mut report,
    );
    validate_normalized_alerts(object.get("alerts"), limits, &mut report);

    let unsafe_paths = collect_unsafe_string_paths(value, report.remaining_error_capacity());
    for path in unsafe_paths {
        report.error(
            path.clone(),
            format!("unsafe UI/code content detected at {path}"),
        );
    }

    report
}
