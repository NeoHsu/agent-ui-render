use std::{collections::BTreeSet, error::Error, io, path::PathBuf, process::Command};

use agent_ui_render_core::{
    Limits, RuntimeConfig, ThemeTokens, ValidationOptions,
    chart::{bar_orientation_for_view, chart_kind_for_view},
    domain, is_safe_css_color_value,
    markdown::markdown_to_html,
    normalize_report, plan_ui_spec,
    render::{render_static_html_with_theme_tokens, render_theme_token_css},
    render_static_html, render_vue_html_shell, validate_report, validate_report_with_options,
    wire::{compact, v2},
};
use serde_json::{Value, json};

const EXAMPLES: &[(&str, &str)] = &[
    (
        "comparison-report.input.json",
        include_str!("../../../examples/comparison-report.input.json"),
    ),
    (
        "component-showcase.input.json",
        include_str!("../../../examples/component-showcase.input.json"),
    ),
    (
        "component-showcase-minimal.input.json",
        include_str!("../../../examples/component-showcase-minimal.input.json"),
    ),
    (
        "revenue-overview.input.json",
        include_str!("../../../examples/revenue-overview.input.json"),
    ),
    (
        "risk-summary.input.json",
        include_str!("../../../examples/risk-summary.input.json"),
    ),
];

const COMPACT_SCHEMA: &str = include_str!("../../../schemas/v1/compact.schema.json");
const NORMALIZED_SCHEMA: &str = include_str!("../../../schemas/v1/normalized.schema.json");
const SPEC_SCHEMA: &str = include_str!("../../../schemas/v1/spec.schema.json");
const CONFIG_SCHEMA: &str = include_str!("../../../schemas/config.schema.json");
const COMPACT_V2_SCHEMA: &str = include_str!("../../../schemas/v2/compact.schema.json");
const NORMALIZED_V2_SCHEMA: &str = include_str!("../../../schemas/v2/normalized.schema.json");
const SPEC_V2_SCHEMA: &str = include_str!("../../../schemas/v2/spec.schema.json");
const V2_SHOWCASE: &str = include_str!("../../../examples/v2-chart-showcase.input.json");
const MARKDOWN_SECURITY_CASES: &str = include_str!("../../../fixtures/markdown-security.json");

fn schema_validator(source: &str) -> Result<jsonschema::Validator, Box<dyn Error>> {
    let schema: Value = serde_json::from_str(source)?;
    Ok(jsonschema::validator_for(&schema)?)
}

fn assert_schema_valid(validator: &jsonschema::Validator, name: &str, value: &Value) {
    let errors = validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "{name}: schema errors: {errors:#?}");
}

fn strings_at<'a>(schema: &'a Value, pointer: &str) -> Vec<&'a str> {
    schema
        .pointer(pointer)
        .unwrap_or_else(|| panic!("missing schema pointer {pointer}"))
        .as_array()
        .unwrap_or_else(|| panic!("schema pointer {pointer} is not an array"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("schema pointer {pointer} contains non-string"))
        })
        .collect()
}

fn workspace_root() -> Result<PathBuf, io::Error> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("workspace root should exist"))
}

#[path = "core/charts.rs"]
mod charts;
#[path = "core/compact.rs"]
mod compact_tests;
#[path = "core/rendering.rs"]
mod rendering;
#[path = "core/schemas.rs"]
mod schemas;
#[path = "core/validation.rs"]
mod validation;
