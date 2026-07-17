use super::*;

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
import {{ parse }} from "vega";
import {{ compile }} from "vega-lite";
const report = {report};
const charts = report.views.filter((view) => view.intent === "chart");
for (const chart of charts) parse(compile(chart.spec).spec);
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
