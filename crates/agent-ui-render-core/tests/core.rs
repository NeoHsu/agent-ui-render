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

#[test]
fn validates_and_normalizes_revenue_overview() -> Result<(), Box<dyn Error>> {
    let payload: Value = serde_json::from_str(include_str!(
        "../../../examples/revenue-overview.input.json"
    ))?;
    let report = validate_report(&payload);
    assert!(report.errors.is_empty(), "{:#?}", report.errors);

    let normalized = normalize_report(&payload)?.input;
    assert_eq!(normalized.schema, domain::NORMALIZED_SCHEMA);
    assert_eq!(normalized.version, domain::FORMAT_VERSION);
    assert_eq!(normalized.title.as_deref(), Some("Revenue Overview"));
    assert!(normalized.datasets.contains_key("sales"));
    let view = normalized
        .views
        .first()
        .ok_or_else(|| io::Error::other("normalized report should include a view"))?;
    assert_eq!(view.intent, domain::VIEW_INTENT_TREND);
    Ok(())
}

#[test]
fn rejects_unsafe_content() {
    let payload = json!({
        "version": 1,
        "t": "Bad <script>alert(1)</script>"
    });
    let report = validate_report(&payload);
    assert!(!report.errors.is_empty());
    assert!(
        report
            .errors
            .iter()
            .any(|finding| finding.message.contains("unsafe UI/code content"))
    );
}

#[test]
fn rejects_security_adversarial_strings() {
    let cases = [
        ("html tag", "<img src=x onerror=alert(1)>"),
        ("javascript href", "[x](javascript:alert(1))"),
        ("dom event assignment", "onclick = alert(1)"),
        ("camel event assignment", "onClick: runExploit"),
        ("react escape hatch", "dangerouslySetInnerHTML: html"),
        ("ui style assignment", "style: position:fixed"),
        ("component injection", "componentName = EvilWidget"),
    ];

    for (name, text) in cases {
        let payload = json!({ "version": 1, "t": text });
        let report = validate_report(&payload);
        assert!(
            report
                .errors
                .iter()
                .any(|finding| finding.message.contains("unsafe UI/code content")),
            "{name}: {:#?}",
            report.errors
        );
    }
}

#[test]
fn markdown_renderer_escapes_or_drops_unsafe_inline_content() {
    let html = markdown_to_html(
        "# Safe\n\n<script>alert(1)</script> [bad](javascript:alert(1)) {warning: pending}",
    );
    assert!(!html.contains("<script>"));
    assert!(!html.contains("javascript:"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("<span class=\"semantic semantic-warning\">pending</span>"));
}

#[test]
fn plans_chart_and_table_blocks() -> Result<(), Box<dyn Error>> {
    let payload: Value = serde_json::from_str(include_str!(
        "../../../examples/revenue-overview.input.json"
    ))?;
    let normalized = normalize_report(&payload)?.input;
    let spec = plan_ui_spec(&normalized);
    assert_eq!(spec["schema"], domain::SPEC_SCHEMA);
    assert_eq!(spec["version"], domain::FORMAT_VERSION);
    let blocks = spec["blocks"]
        .as_array()
        .ok_or_else(|| io::Error::other("spec blocks should be an array"))?;
    assert!(blocks.iter().any(|block| block["type"] == "chart"));
    assert!(blocks.iter().any(|block| block["type"] == "table"));
    Ok(())
}

#[test]
fn examples_pass_rust_validator_json_schemas_plan_and_render() -> Result<(), Box<dyn Error>> {
    let compact_validator = schema_validator(COMPACT_SCHEMA)?;
    let normalized_validator = schema_validator(NORMALIZED_SCHEMA)?;
    let spec_validator = schema_validator(SPEC_SCHEMA)?;

    for (name, source) in EXAMPLES {
        let payload: Value = serde_json::from_str(source).unwrap_or_else(|error| {
            panic!("{name}: failed to parse fixture JSON: {error}");
        });
        let report = validate_report(&payload);
        assert!(report.errors.is_empty(), "{name}: {:#?}", report.errors);
        assert_schema_valid(&compact_validator, name, &payload);

        let normalized = normalize_report(&payload)
            .unwrap_or_else(|error| panic!("{name}: failed to normalize: {error}"))
            .input;
        let normalized_value = serde_json::to_value(&normalized)?;
        assert_schema_valid(
            &normalized_validator,
            &format!("normalized {name}"),
            &normalized_value,
        );

        let spec = plan_ui_spec(&normalized);
        assert_eq!(spec["schema"], domain::SPEC_SCHEMA, "{name}");
        assert_eq!(spec["version"], domain::FORMAT_VERSION, "{name}");
        assert_schema_valid(&spec_validator, &format!("spec {name}"), &spec);
        assert!(
            render_static_html(&normalized).contains("agent-ui-render"),
            "{name}"
        );
        assert!(
            render_vue_html_shell(&normalized).contains("agent-ui-payload"),
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn compact_v2_covers_non_geo_non_image_chart_families() -> Result<(), Box<dyn Error>> {
    let compact_validator = schema_validator(COMPACT_V2_SCHEMA)?;
    let normalized_validator = schema_validator(NORMALIZED_V2_SCHEMA)?;
    let spec_validator = schema_validator(SPEC_V2_SCHEMA)?;
    let payload: Value = serde_json::from_str(V2_SHOWCASE)?;

    let validation = validate_report(&payload);
    assert!(validation.errors.is_empty(), "{:#?}", validation.errors);
    assert_schema_valid(&compact_validator, "v2 showcase", &payload);

    let normalized = normalize_report(&payload)?.input;
    assert_eq!(normalized.version, domain::FORMAT_VERSION_V2);
    assert_eq!(normalized.views.len(), 45);
    assert!(
        normalized
            .views
            .iter()
            .filter(|view| view.intent == domain::VIEW_INTENT_CHART)
            .all(|view| view.spec.is_some() && view.chart.is_some())
    );
    let normalized_value = serde_json::to_value(&normalized)?;
    assert_schema_valid(
        &normalized_validator,
        "normalized v2 showcase",
        &normalized_value,
    );

    let interactions = [
        (0, "agent_hover"),
        (4, "agent_select"),
        (10, "agent_legend"),
        (42, "agent_brush"),
        (43, "agent_zoom"),
    ];
    for (index, name) in interactions {
        let chart_spec = normalized.views[index]
            .spec
            .as_ref()
            .ok_or_else(|| io::Error::other("showcase chart should have a Vega-Lite spec"))?;
        assert_eq!(chart_spec["params"][0]["name"], name);
        if name != "agent_zoom" {
            assert_eq!(
                chart_spec["encoding"]["opacity"]["condition"]["param"],
                name
            );
            assert_eq!(chart_spec["encoding"]["opacity"]["value"], 0.4);
        }
    }
    assert_eq!(
        normalized.views[10]
            .spec
            .as_ref()
            .ok_or_else(|| io::Error::other("density chart should have a Vega-Lite spec"))?["params"]
            [0]["select"]["fields"][0],
        "group"
    );
    let line_spec = normalized.views[0]
        .spec
        .as_ref()
        .ok_or_else(|| io::Error::other("line chart should have a Vega-Lite spec"))?;
    assert!(
        line_spec["encoding"]["color"]["legend"]["labelExpr"]
            .as_str()
            .is_some_and(|expression| expression.contains("Value B"))
    );
    assert_eq!(
        normalized.views[2]
            .spec
            .as_ref()
            .ok_or_else(|| io::Error::other("trail chart should have a Vega-Lite spec"))?["encoding"]
            ["size"]["legend"]["tickCount"],
        3
    );
    assert_eq!(
        normalized.views[9]
            .spec
            .as_ref()
            .ok_or_else(|| io::Error::other("histogram should have a Vega-Lite spec"))?["encoding"]
            ["x"]["title"],
        "Value"
    );
    assert_eq!(
        normalized.views[41]
            .spec
            .as_ref()
            .ok_or_else(|| io::Error::other("2D histogram should have a Vega-Lite spec"))?["encoding"]
            ["color"]["legend"]["format"],
        "d"
    );

    let spec = plan_ui_spec(&normalized);
    assert_eq!(spec["version"], domain::FORMAT_VERSION_V2);
    assert_schema_valid(&spec_validator, "v2 planned spec", &spec);
    assert!(render_vue_html_shell(&normalized).contains("vega-chart"));
    assert!(render_static_html(&normalized).contains("Interactive line chart"));

    let compact_schema: Value = serde_json::from_str(COMPACT_V2_SCHEMA)?;
    assert_eq!(
        strings_at(&compact_schema, "/$defs/chartCode/enum"),
        v2::CHART_CODES
    );
    assert!(!v2::CHART_CODES.contains(&"image"));
    assert!(!v2::CHART_CODES.contains(&"geoshape"));

    let compile_script = format!(
        r#"
import {{ compile }} from "vega-lite";
const report = {report};
const charts = report.views.filter((view) => view.intent === "chart");
for (const chart of charts) compile(chart.spec);
console.log(charts.length);
"#,
        report = serde_json::to_string(&normalized)?,
    );
    // Resolve vega-lite and its vega peer from the renderer's installed,
    // lockfile-pinned node_modules; the workspace root has none in CI.
    let output = Command::new("bun")
        .arg("--eval")
        .arg(compile_script)
        .current_dir(workspace_root()?.join("renderer-vue"))
        .output()?;
    assert!(
        output.status.success(),
        "Vega-Lite compile smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "44");
    Ok(())
}

#[test]
fn compact_v2_rejects_excluded_and_raw_vega_capabilities() {
    for code in ["image", "isotype", "map", "geoshape"] {
        let payload = json!({
            "version": 2,
            "d": [["data", [["x", "n"]], [[1]]]],
            "v": [[code, 0, 0]]
        });
        let validation = validate_report(&payload);
        assert!(
            !validation.errors.is_empty(),
            "excluded code {code} was accepted"
        );
    }
    let payload = json!({
        "version": 2,
        "d": [["data", [["x", "n"]], [[1]]]],
        "v": [["ln", 0, 0, [0], {"mark": "image"}]]
    });
    assert!(!validate_report(&payload).errors.is_empty());
}

#[test]
fn compact_insights_assumptions_and_metric_delta_flow() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "t": "Narrative Extras",
        "d": [[
            "sales",
            [["month", "s"], ["revenue", "cur", "USD"]],
            [["Jan", 120000], ["Feb", 135000]]
        ]],
        "m": [
            ["Revenue", 135000, "cur", "USD", [0.125, "pct"]],
            ["Open Bugs", 42, "n", null, -3],
            ["Coverage", 0.87, "pct", null, 0]
        ],
        "i": ["Revenue grew 12.5% month over month."],
        "as": ["February totals exclude returns."],
        "v": [["r", 0]]
    });

    let validation = validate_report(&payload);
    assert!(validation.errors.is_empty(), "{:#?}", validation.errors);
    assert!(validation.warnings.is_empty(), "{:#?}", validation.warnings);
    let compact_validator = schema_validator(COMPACT_SCHEMA)?;
    assert_schema_valid(&compact_validator, "compact narrative extras", &payload);

    let normalized = normalize_report(&payload)?.input;
    assert_eq!(
        normalized.insights,
        vec!["Revenue grew 12.5% month over month."]
    );
    assert_eq!(
        normalized.assumptions,
        vec!["February totals exclude returns."]
    );
    let delta = |index: usize| {
        normalized.metrics[index]
            .delta
            .as_ref()
            .ok_or_else(|| io::Error::other("metric delta should exist"))
    };
    let revenue = delta(0)?;
    assert_eq!(revenue.label.as_deref(), Some("+12.5%"));
    assert_eq!(
        revenue.direction.as_deref(),
        Some(domain::DELTA_DIRECTION_UP)
    );
    assert_eq!(revenue.format.as_deref(), Some("percent"));
    let bugs = delta(1)?;
    assert_eq!(bugs.label.as_deref(), Some("-3"));
    assert_eq!(
        bugs.direction.as_deref(),
        Some(domain::DELTA_DIRECTION_DOWN)
    );
    assert_eq!(
        delta(2)?.direction.as_deref(),
        Some(domain::DELTA_DIRECTION_FLAT)
    );

    let normalized_value = serde_json::to_value(&normalized)?;
    let normalized_validator = schema_validator(NORMALIZED_SCHEMA)?;
    assert_schema_valid(
        &normalized_validator,
        "normalized narrative extras",
        &normalized_value,
    );

    let spec = plan_ui_spec(&normalized);
    let spec_validator = schema_validator(SPEC_SCHEMA)?;
    assert_schema_valid(&spec_validator, "planned narrative extras", &spec);
    let blocks = spec["blocks"]
        .as_array()
        .ok_or_else(|| io::Error::other("spec blocks should exist"))?;
    assert!(
        blocks
            .iter()
            .any(|block| block["type"] == "metric" && block["delta"]["label"] == "+12.5%")
    );
    assert!(blocks.iter().any(|block| block["variant"] == "insight"));
    assert!(blocks.iter().any(|block| block["variant"] == "assumption"));

    assert!(render_vue_html_shell(&normalized).contains("Revenue grew 12.5% month over month."));
    let static_html = render_static_html(&normalized);
    assert!(static_html.contains("Key insights"));
    assert!(static_html.contains("Assumptions"));
    assert!(static_html.contains("+12.5%"));

    let bad_delta_format = json!({"version": 1, "m": [["A", 1, "n", null, [0.1, "cur"]]]});
    assert!(!validate_report(&bad_delta_format).errors.is_empty());
    let bad_delta_shape = json!({"version": 1, "m": [["A", 1, "n", null, "big"]]});
    assert!(!validate_report(&bad_delta_shape).errors.is_empty());
    let bad_insight = json!({"version": 1, "i": ["ok", 42]});
    assert!(!validate_report(&bad_insight).errors.is_empty());
    Ok(())
}

#[test]
fn schema_enums_match_centralized_code_mappings() -> Result<(), Box<dyn Error>> {
    let compact_schema: Value = serde_json::from_str(COMPACT_SCHEMA)?;
    assert_eq!(
        strings_at(&compact_schema, "/$defs/typeCode/anyOf/0/enum"),
        compact::BASE_TYPE_CODES
    );
    assert_eq!(
        strings_at(&compact_schema, "/$defs/viewCode/enum"),
        compact::VIEW_CODES
    );
    assert_eq!(
        strings_at(&compact_schema, "/$defs/view/oneOf/0/prefixItems/0/enum"),
        compact::SIMPLE_VIEW_CODES
    );
    assert_eq!(
        strings_at(&compact_schema, "/$defs/view/oneOf/1/prefixItems/0/enum"),
        compact::MEASURE_VIEW_CODES
    );
    assert_eq!(
        compact_schema
            .pointer("/$defs/view/oneOf/2/prefixItems/0/const")
            .and_then(Value::as_str),
        Some(compact::VIEW_CODE_DISTRIBUTION)
    );
    assert_eq!(
        compact_schema
            .pointer("/$defs/view/oneOf/3/prefixItems/0/const")
            .and_then(Value::as_str),
        Some(compact::VIEW_CODE_RECORDS)
    );
    assert_eq!(
        strings_at(&compact_schema, "/$defs/alert/prefixItems/0/enum"),
        compact::ALERT_LEVEL_CODES
    );
    assert_eq!(
        strings_at(
            &compact_schema,
            "/$defs/metricDelta/oneOf/1/prefixItems/1/enum"
        ),
        compact::DELTA_FORMAT_CODES
    );
    let compact_v2_schema: Value = serde_json::from_str(COMPACT_V2_SCHEMA)?;
    assert_eq!(
        strings_at(
            &compact_v2_schema,
            "/$defs/metricDelta/oneOf/1/prefixItems/1/enum"
        ),
        compact::DELTA_FORMAT_CODES
    );

    let normalized_schema: Value = serde_json::from_str(NORMALIZED_SCHEMA)?;
    assert_eq!(
        strings_at(&normalized_schema, "/properties/theme/enum"),
        domain::THEMES
    );
    assert_eq!(
        strings_at(&normalized_schema, "/$defs/columnType/enum"),
        domain::COLUMN_TYPES
    );
    assert_eq!(
        strings_at(&normalized_schema, "/$defs/metric/properties/format/enum"),
        domain::METRIC_FORMATS
    );
    assert_eq!(
        strings_at(
            &normalized_schema,
            "/$defs/metricDelta/properties/direction/enum"
        ),
        domain::DELTA_DIRECTIONS
    );
    assert_eq!(
        strings_at(
            &normalized_schema,
            "/$defs/metricDelta/properties/format/enum"
        ),
        domain::DELTA_FORMATS
    );
    assert_eq!(
        strings_at(
            &normalized_schema,
            "/$defs/viewIntent/properties/intent/enum"
        ),
        domain::VIEW_INTENTS_V1
    );
    assert_eq!(
        strings_at(&normalized_schema, "/$defs/alert/properties/level/enum"),
        domain::ALERT_LEVELS
    );

    let spec_schema: Value = serde_json::from_str(SPEC_SCHEMA)?;
    assert_eq!(
        strings_at(&spec_schema, "/properties/theme/enum"),
        domain::THEMES
    );
    assert_eq!(
        strings_at(&spec_schema, "/$defs/columnType/enum"),
        domain::COLUMN_TYPES
    );
    assert_eq!(
        strings_at(&spec_schema, "/$defs/alertBlock/properties/level/enum"),
        domain::ALERT_LEVELS
    );

    let config_schema: Value = serde_json::from_str(CONFIG_SCHEMA)?;
    let config_validator = schema_validator(CONFIG_SCHEMA)?;
    assert_schema_valid(
        &config_validator,
        "theme token config",
        &json!({
            "themeTokens": {
                "primary": "#8b5cf6",
                "series1": "oklch(62% 0.2 275)"
            }
        }),
    );
    let schema_theme_tokens = config_schema["$defs"]["themeTokens"]["properties"]
        .as_object()
        .ok_or_else(|| io::Error::other("config schema theme token properties should exist"))?
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let runtime_theme_tokens = ThemeTokens::KEYS.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(schema_theme_tokens, runtime_theme_tokens);
    Ok(())
}

#[test]
fn theme_tokens_render_as_safe_css_overrides() {
    let tokens = ThemeTokens {
        page: Some("#0b1220".to_owned()),
        text: Some("rgb(249 250 251)".to_owned()),
        primary: Some("#8b5cf6".to_owned()),
        series_1: Some("oklch(62% 0.2 275)".to_owned()),
        ..ThemeTokens::default()
    };
    assert!(tokens.validate().is_ok());

    let css = render_theme_token_css(&tokens);
    assert!(css.contains("--agent-primary: #8b5cf6;"));
    assert!(css.contains("--agent-series-1: oklch(62% 0.2 275);"));
    assert!(css.contains("body.agent-ui-standalone.agent-ui-standalone[data-theme]"));
    assert!(css.contains("background: var(--agent-page);"));
    assert!(css.contains("color: var(--agent-text);"));

    let report = domain::Report {
        title: Some("Brand Report".to_owned()),
        ..domain::Report::default()
    };
    let html = render_static_html_with_theme_tokens(&report, &tokens);
    assert!(html.contains("Brand Report"));
    assert!(html.contains("--agent-primary: #8b5cf6;"));
}

#[test]
fn theme_token_values_fail_closed() -> Result<(), Box<dyn Error>> {
    assert!(is_safe_css_color_value("#fff"));
    assert!(is_safe_css_color_value("rgba(15, 23, 42, 0.8)"));
    assert!(!is_safe_css_color_value("#12"));
    assert!(!is_safe_css_color_value(
        "#fff;}</style><script>bad()</script>"
    ));

    let config: RuntimeConfig = serde_json::from_value(json!({
        "themeTokens": {
            "primary": "#fff; background: red"
        }
    }))?;
    let error = config.validate().expect_err("unsafe token should fail");
    assert_eq!(error.violations()[0].key, "primary");

    let css = render_theme_token_css(&config.theme_tokens);
    assert!(!css.contains("--agent-primary"));
    assert!(!css.contains("background: red"));
    Ok(())
}

#[test]
fn payload_size_limits_cover_core_guardrails() {
    let payload = json!({
        "version": 1,
        "t": "abcd",
        "d": [
            ["one", [["first", "s"], ["second", "s"]], [["toolong", "x"], ["y", "z"]]],
            ["two", [["value", "n"]], [[1]]]
        ],
        "md": [["title", "abcdef"], ["other", "abcdef"]]
    });
    let options = ValidationOptions {
        limits: Limits {
            max_datasets: 1,
            max_columns_per_dataset: 1,
            max_rows_per_dataset: 1,
            max_cells_per_dataset: 1,
            max_string_chars: 3,
            max_markdown_sections: 1,
            max_markdown_section_chars: 5,
            max_total_markdown_chars: 8,
            ..Limits::default()
        },
    };

    let report = validate_report_with_options(&payload, &options);
    let messages = report
        .errors
        .iter()
        .map(|finding| format!("{}: {}", finding.path, finding.message))
        .collect::<Vec<_>>()
        .join("\n");
    for expected in [
        "datasets count 2 exceeds max 1",
        "columns count 2 exceeds max 1",
        "rows count 2 exceeds max 1",
        "cells count 4 exceeds max 1",
        "field 't' length 4 chars exceeds max 3",
        "markdown sections count 2 exceeds max 1",
        "markdown entry length 6 chars exceeds max 5",
        "total markdown length 12 chars exceeds max 8",
    ] {
        assert!(
            messages.contains(expected),
            "missing {expected:?} in\n{messages}"
        );
    }
}

#[test]
fn hostile_json_shapes_and_large_payloads_do_not_panic() {
    let deep_array = (0..64).fold(json!("leaf"), |value, _| json!([value]));
    let wide_rows = (0..250)
        .map(|index| json!([format!("row_{index}"), index]))
        .collect::<Vec<_>>();
    let cases = vec![
        Value::Null,
        json!([]),
        json!({"version": 1, "d": "not arrays", "v": [{"bad": true}], "a": [42]}),
        json!({"version": 1, "d": [["broken", [], [deep_array]]]}),
        json!({
            "version": 1,
            "d": [["huge", [["name", "s"], ["value", "n"]], wide_rows]],
            "v": [["b", 0, 0, [1]]],
            "md": [["Huge", "x".repeat(10_000)]]
        }),
    ];

    for payload in cases {
        let result = std::panic::catch_unwind(|| {
            let report = validate_report(&payload);
            let constrained = validate_report_with_options(
                &payload,
                &ValidationOptions {
                    limits: Limits {
                        max_rows_per_dataset: 10,
                        max_string_chars: 128,
                        max_markdown_section_chars: 256,
                        ..Limits::default()
                    },
                },
            );
            if report.errors.is_empty()
                && constrained.errors.is_empty()
                && let Ok(normalized) = normalize_report(&payload)
            {
                let _ = plan_ui_spec(&normalized.input);
                let _ = render_static_html(&normalized.input);
            }
        });
        assert!(result.is_ok(), "hostile payload caused panic: {payload}");
    }
}

#[test]
fn golden_normalize_and_plan_small_report() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "t": "Mini",
        "s": "Short summary",
        "d": [[
            "sales",
            [["month", "s"], ["revenue", "cur", "USD", "Revenue"]],
            [["Jan", 100], ["Feb", 120]]
        ]],
        "m": [["Revenue", 120, "cur", "USD"]],
        "md": [["Notes", "**ok**"]],
        "v": [["t", 0, 0, [1]], ["r", 0]],
        "a": [["i", "Ready"]]
    });

    let normalized = normalize_report(&payload)?.input;
    assert_eq!(
        serde_json::to_value(&normalized)?,
        json!({
            "schema": "ui.input.normalized",
            "version": 1,
            "title": "Mini",
            "summary": "Short summary",
            "datasets": {
                "sales": {
                    "columns": [
                        {"key": "month", "label": "Month", "type": "string"},
                        {"key": "revenue", "label": "Revenue", "type": "currency", "unit": "USD"}
                    ],
                    "rows": [["Jan", 100], ["Feb", 120]]
                }
            },
            "metrics": [{"label": "Revenue", "value": 120, "format": "currency", "unit": "USD"}],
            "markdown": [{"title": "Notes", "content": "**ok**"}],
            "views": [
                {"intent": "trend", "data": "sales", "x": "month", "measures": ["revenue"]},
                {"intent": "precise_records", "data": "sales"}
            ],
            "alerts": [{"level": "info", "content": "Ready"}]
        })
    );

    assert_eq!(
        plan_ui_spec(&normalized),
        json!({
            "schema": "ui.spec",
            "version": 1,
            "title": "Mini",
            "summary": "Short summary",
            "datasets": {
                "sales": {
                    "columns": [
                        {"key": "month", "label": "Month", "type": "string"},
                        {"key": "revenue", "label": "Revenue", "type": "currency", "unit": "USD"}
                    ],
                    "rows": [["Jan", 100], ["Feb", 120]]
                }
            },
            "blocks": [
                {"id": "metric_revenue", "type": "metric", "label": "Revenue", "value": 120, "format": "currency", "unit": "USD"},
                {"id": "markdown_1", "type": "markdown", "content": "**ok**", "title": "Notes"},
                {"id": "chart_sales_trend_1", "type": "chart", "chart": "line", "data": "sales", "x": "month", "y": ["revenue"]},
                {"id": "table_sales", "type": "table", "data": "sales"},
                {"id": "alert_1", "type": "alert", "level": "info", "content": "Ready"}
            ]
        })
    );
    Ok(())
}

#[test]
fn compact_records_view_projects_table_columns() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "d": [[
            "actions",
            [["action", "s"], ["owner", "s"], ["status", "s"]],
            [["Add guardrail", "Platform", "planned"], ["Verify alert", "SRE", "done"]]
        ]],
        "v": [["r", 0, [0, 2]]]
    });
    assert_schema_valid(
        &schema_validator(COMPACT_SCHEMA)?,
        "projected records",
        &payload,
    );
    let report = validate_report(&payload);
    assert!(report.errors.is_empty(), "{:#?}", report.errors);

    let normalized = normalize_report(&payload)?.input;
    assert_eq!(
        normalized.views[0].columns.as_deref(),
        Some(&["action".to_owned(), "status".to_owned()][..])
    );

    let spec = plan_ui_spec(&normalized);
    assert_eq!(spec["blocks"][0]["columns"], json!(["action", "status"]));

    let html = render_static_html(&normalized);
    assert!(html.contains("<th>Action</th>"));
    assert!(html.contains("<th>Status</th>"));
    assert!(!html.contains("<th>Owner</th>"));
    Ok(())
}

#[test]
fn normalizes_dictionary_and_column_major_data() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "dict": { "region": ["North", "South"] },
        "d": [
            [
                "sales",
                "cols",
                [["region", "dict:region"], ["revenue", "cur", "USD"]],
                [[0, 1], [1200, 900]]
            ]
        ],
        "v": [["b", 0, 0, [1]]]
    });
    let report = validate_report(&payload);
    assert!(report.errors.is_empty(), "{:#?}", report.errors);
    let normalized = normalize_report(&payload)?.input;
    let dataset = normalized
        .datasets
        .get("sales")
        .ok_or_else(|| io::Error::other("normalized report should include sales dataset"))?;
    assert_eq!(dataset.rows[0][0], json!("North"));
    assert_eq!(dataset.rows[1][0], json!("South"));
    assert_eq!(dataset.rows[0][1], json!(1200));
    assert_eq!(normalized.views[0].x.as_deref(), Some("region"));
    assert_eq!(
        normalized.views[0].measures.as_deref(),
        Some(&["revenue".to_owned()][..])
    );
    Ok(())
}

#[test]
fn composition_chart_falls_back_from_pie_for_many_categories() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "d": [
            [
                "share",
                [["segment", "s"], ["value", "n"]],
                [["A", 1], ["B", 2], ["C", 3], ["D", 4], ["E", 5], ["F", 6]]
            ]
        ],
        "v": [["p", 0, 0, [1]]]
    });
    let normalized = normalize_report(&payload)?.input;
    let dataset = normalized
        .datasets
        .get("share")
        .ok_or_else(|| io::Error::other("normalized report should include share dataset"))?;
    assert_eq!(chart_kind_for_view(&normalized.views[0], dataset), "bar");
    Ok(())
}

#[test]
fn grouped_bar_orientation_and_static_rendering_are_complete() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "d": [
            [
                "quarters",
                [["quarter", "s"], ["revenue", "cur", "USD"], ["profit", "cur", "USD"]],
                [["Q1", 1030000, 260000], ["Q2", 1140000, 310000], ["Q3", 1210000, 340000], ["Q4", 1340000, 370000]]
            ],
            [
                "channels",
                [["channel", "s"], ["spend", "cur", "USD"], ["pipeline", "cur", "USD"]],
                [["Paid Search", 62000, 286000], ["Events", 48000, 254000], ["Partners", 27000, 191000]]
            ]
        ],
        "v": [["b", 0, 0, [1, 2]], ["b", 1, 0, [1, 2]]]
    });
    let report = validate_report(&payload);
    assert!(report.errors.is_empty(), "{:#?}", report.errors);
    let normalized = normalize_report(&payload)?.input;
    let quarters = normalized
        .datasets
        .get("quarters")
        .ok_or_else(|| io::Error::other("quarters dataset should exist"))?;
    let channels = normalized
        .datasets
        .get("channels")
        .ok_or_else(|| io::Error::other("channels dataset should exist"))?;
    assert_eq!(
        bar_orientation_for_view(&normalized.views[0], quarters),
        "vertical"
    );
    assert_eq!(
        bar_orientation_for_view(&normalized.views[1], channels),
        "horizontal"
    );

    let spec = plan_ui_spec(&normalized);
    assert_eq!(spec["blocks"][0]["orientation"], "vertical");
    assert_eq!(spec["blocks"][1]["orientation"], "horizontal");

    let html = render_static_html(&normalized);
    assert!(html.contains("vertical-bar-chart"));
    assert!(html.contains("bar-axis"));
    assert!(html.contains("Revenue"));
    assert!(html.contains("Profit"));
    assert!(html.contains("Spend"));
    assert!(html.contains("Pipeline"));
    Ok(())
}

#[test]
fn rust_and_vue_chart_and_markdown_parity() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "version": 1,
        "d": [[
            "share",
            [["segment", "s"], ["value", "n"], ["latency", "n"]],
            [["Q1", 3, 120], ["Q2", 2, 180], ["Q3", 1, 220]]
        ]],
        "v": [
            ["t", 0, 0, [1]],
            ["s", 0, 2, [1]],
            ["p", 0, 0, [1]],
            ["b", 0, 0, [1]]
        ]
    });
    let normalized = normalize_report(&payload)?.input;
    let dataset = normalized
        .datasets
        .get("share")
        .ok_or_else(|| io::Error::other("normalized report should include share dataset"))?;
    let rust_charts = normalized
        .views
        .iter()
        .map(|view| chart_kind_for_view(view, dataset).to_owned())
        .collect::<Vec<_>>();
    let rust_orientations = normalized
        .views
        .iter()
        .map(|view| bar_orientation_for_view(view, dataset).to_owned())
        .collect::<Vec<_>>();
    let markdown_samples = vec![
        "# Heading\n\nParagraph with **strong**, *em*, `code`, {warning: pending}, and [guide](https://example.com/report-guide).",
        "> quoted note\n\n- one\n- two\n\n1. first\n2. second\n\n```sql\nselect 1;\n```",
    ];
    let rust_markdown = markdown_samples
        .iter()
        .map(|sample| markdown_to_html(sample))
        .collect::<Vec<_>>();

    let script = format!(
        r#"
import {{ barOrientationForView, chartKindForView }} from "./renderer-vue/src/chart-selection.ts";
import {{ markdownToHtml }} from "./renderer-vue/src/markdown.ts";
const dataset = {dataset};
const views = {views};
const markdown = {markdown};
console.log(JSON.stringify({{
  charts: views.map((view) => chartKindForView(view, dataset)),
  orientations: views.map((view) => barOrientationForView(view, dataset)),
  markdown: markdown.map((sample) => markdownToHtml(sample)),
}}));
"#,
        dataset = serde_json::to_string(dataset)?,
        views = serde_json::to_string(&normalized.views)?,
        markdown = serde_json::to_string(&markdown_samples)?,
    );
    let output = Command::new("bun")
        .arg("--eval")
        .arg(script)
        .current_dir(workspace_root()?)
        .output()?;
    assert!(
        output.status.success(),
        "bun parity script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let vue: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(vue["charts"], json!(rust_charts));
    assert_eq!(vue["orientations"], json!(rust_orientations));
    assert_eq!(vue["markdown"], json!(rust_markdown));
    Ok(())
}

#[test]
fn renders_vue_and_static_html() -> Result<(), Box<dyn Error>> {
    let payload: Value = serde_json::from_str(include_str!(
        "../../../examples/revenue-overview.input.json"
    ))?;
    let normalized = normalize_report(&payload)?.input;
    let vue_html = render_vue_html_shell(&normalized);
    assert!(vue_html.contains("agent-ui-root"));
    assert!(vue_html.contains("agent-ui-payload"));
    assert!(vue_html.contains("Revenue Overview"));

    let static_html = render_static_html(&normalized);
    assert!(static_html.contains("Revenue Overview"));
    assert!(static_html.contains("<table>"));
    Ok(())
}

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
