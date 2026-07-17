use super::*;

#[test]
fn validates_and_normalizes_revenue_overview() -> Result<(), Box<dyn Error>> {
    let payload: Value = serde_json::from_str(include_str!(
        "../../../../examples/revenue-overview.input.json"
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
            max_total_rows: 1,
            max_total_cells: 1,
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
        "total rows count 2 exceeds max 1",
        "total cells count 4 exceeds max 1",
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
fn diagnostic_budget_bounds_adversarial_payloads() {
    let payload = json!({
        "version": 1,
        "d": [[
            "rows",
            [["value", "s"]],
            (0..10_000).map(|_| json!([])).collect::<Vec<_>>()
        ]]
    });
    let options = ValidationOptions {
        limits: Limits {
            max_rows_per_dataset: 20_000,
            max_findings: 10,
            ..Limits::default()
        },
    };

    let report = validate_report_with_options(&payload, &options);
    assert!(!report.is_ok());
    assert!(report.errors.len() <= 11, "{:#?}", report.errors);
    assert!(
        report
            .errors
            .iter()
            .any(|finding| finding.message.contains("diagnostic limit 10 reached")),
        "{:#?}",
        report.errors
    );
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
