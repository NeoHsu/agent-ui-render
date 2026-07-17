use super::*;

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
        "../../../../examples/revenue-overview.input.json"
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
