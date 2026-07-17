use super::*;

#[test]
fn plans_chart_and_table_blocks() -> Result<(), Box<dyn Error>> {
    let payload: Value = serde_json::from_str(include_str!(
        "../../../../examples/revenue-overview.input.json"
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
