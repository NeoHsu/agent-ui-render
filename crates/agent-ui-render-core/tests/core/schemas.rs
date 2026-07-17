use super::*;

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
            "documentLanguage": "zh-Hant",
            "themeTokens": {
                "primary": "#8b5cf6",
                "series1": "oklch(62% 0.2 275)"
            }
        }),
    );
    assert!(!config_validator.is_valid(&json!({
        "documentLanguage": "en\"><script>bad()</script>"
    })));
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
