use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

use crate::{
    domain::Report,
    options::{ThemeTokens, is_safe_css_color_value},
};

use super::formatting::{escape_html, script_safe_json};

pub const RENDERER_JS: &str = include_str!("../../../../generated/renderer.js");
pub const RENDERER_CSS: &str = include_str!("../../../../generated/renderer.css");

const VUE_HANDOFF_FILES: &[(&str, &str)] = &[
    (
        "AgentUiRenderer.vue",
        include_str!("../../../../renderer-vue/src/AgentUiRenderer.vue"),
    ),
    (
        "agent-ui.css",
        include_str!("../../../../renderer-vue/src/agent-ui.css"),
    ),
    (
        "chart-model.ts",
        include_str!("../../../../renderer-vue/src/chart-model.ts"),
    ),
    (
        "chart-data.ts",
        include_str!("../../../../renderer-vue/src/chart-data.ts"),
    ),
    (
        "chart-selection.ts",
        include_str!("../../../../renderer-vue/src/chart-selection.ts"),
    ),
    (
        "format.ts",
        include_str!("../../../../renderer-vue/src/format.ts"),
    ),
    (
        "markdown.ts",
        include_str!("../../../../renderer-vue/src/markdown.ts"),
    ),
    (
        "types.ts",
        include_str!("../../../../renderer-vue/src/types.ts"),
    ),
    (
        "vega-theme.ts",
        include_str!("../../../../renderer-vue/src/vega-theme.ts"),
    ),
    (
        "env.d.ts",
        include_str!("../../../../renderer-vue/src/env.d.ts"),
    ),
    (
        "components/AlertList.vue",
        include_str!("../../../../renderer-vue/src/components/AlertList.vue"),
    ),
    (
        "components/AssumptionList.vue",
        include_str!("../../../../renderer-vue/src/components/AssumptionList.vue"),
    ),
    (
        "components/ChartPreview.vue",
        include_str!("../../../../renderer-vue/src/components/ChartPreview.vue"),
    ),
    (
        "components/DataTableBlock.vue",
        include_str!("../../../../renderer-vue/src/components/DataTableBlock.vue"),
    ),
    (
        "components/InsightList.vue",
        include_str!("../../../../renderer-vue/src/components/InsightList.vue"),
    ),
    (
        "components/MarkdownBlock.vue",
        include_str!("../../../../renderer-vue/src/components/MarkdownBlock.vue"),
    ),
    (
        "components/MetricGrid.vue",
        include_str!("../../../../renderer-vue/src/components/MetricGrid.vue"),
    ),
    (
        "components/ReportFooter.vue",
        include_str!("../../../../renderer-vue/src/components/ReportFooter.vue"),
    ),
    (
        "components/ReportHeader.vue",
        include_str!("../../../../renderer-vue/src/components/ReportHeader.vue"),
    ),
    (
        "components/ReportViewBlock.vue",
        include_str!("../../../../renderer-vue/src/components/ReportViewBlock.vue"),
    ),
    (
        "components/charts/VegaLiteChart.vue",
        include_str!("../../../../renderer-vue/src/components/charts/VegaLiteChart.vue"),
    ),
    (
        "composables/useVegaLiteView.ts",
        include_str!("../../../../renderer-vue/src/composables/useVegaLiteView.ts"),
    ),
    (
        "package.json",
        include_str!("../../../../renderer-vue/src/handoff-package.json"),
    ),
    (
        "README.md",
        include_str!("../../../../renderer-vue/src/HANDOFF.md"),
    ),
    (
        "components/charts/BarChartView.vue",
        include_str!("../../../../renderer-vue/src/components/charts/BarChartView.vue"),
    ),
    (
        "components/charts/LineChartView.vue",
        include_str!("../../../../renderer-vue/src/components/charts/LineChartView.vue"),
    ),
    (
        "components/charts/PieChartView.vue",
        include_str!("../../../../renderer-vue/src/components/charts/PieChartView.vue"),
    ),
    (
        "components/charts/ScatterChartView.vue",
        include_str!("../../../../renderer-vue/src/components/charts/ScatterChartView.vue"),
    ),
    (
        "components/charts/VerticalBarChartView.vue",
        include_str!("../../../../renderer-vue/src/components/charts/VerticalBarChartView.vue"),
    ),
];

#[must_use]
pub fn vue_handoff_files() -> &'static [(&'static str, &'static str)] {
    VUE_HANDOFF_FILES
}

#[must_use]
pub fn render_theme_token_css(theme_tokens: &ThemeTokens) -> String {
    let entries = theme_tokens
        .entries()
        .into_iter()
        .filter(|entry| is_safe_css_color_value(entry.value))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return String::new();
    }

    let mut css = String::from(
        "body.agent-ui-standalone.agent-ui-standalone,\nbody.agent-ui-standalone.agent-ui-standalone[data-theme],\n.agent-ui-render.agent-ui-render,\n.agent-ui-render.agent-ui-render[data-theme] {\n",
    );
    for entry in &entries {
        css.push_str("  ");
        css.push_str(entry.css_var);
        css.push_str(": ");
        css.push_str(entry.value.trim());
        css.push_str(";\n");
    }
    css.push_str("}\n");

    let background_var = if token_value_is_safe(&theme_tokens.page) {
        Some("--agent-page")
    } else if token_value_is_safe(&theme_tokens.bg) {
        Some("--agent-bg")
    } else {
        None
    };
    if background_var.is_some() || token_value_is_safe(&theme_tokens.text) {
        css.push_str(
            "\nbody.agent-ui-standalone.agent-ui-standalone,\nbody.agent-ui-standalone.agent-ui-standalone[data-theme] {\n",
        );
        if let Some(background_var) = background_var {
            css.push_str("  background: var(");
            css.push_str(background_var);
            css.push_str(");\n");
        }
        if token_value_is_safe(&theme_tokens.text) {
            css.push_str("  color: var(--agent-text);\n");
        }
        css.push_str("}\n");
    }

    css
}

pub(super) fn render_theme_token_style_block(theme_tokens: &ThemeTokens) -> String {
    theme_token_style_content(theme_tokens)
        .map_or_else(String::new, |content| format!("\n<style>{content}</style>"))
}

pub(super) fn render_content_security_policy(script: Option<&str>, styles: &[&str]) -> String {
    let script_source = script.map_or_else(
        || "'none'".to_owned(),
        // Vega compiles trusted Rust-generated expressions at runtime. Inline
        // scripts remain hash-bound, while external loads stay disabled.
        |source| format!("'sha256-{}' 'unsafe-eval'", content_hash(source)),
    );
    let style_source = if styles.is_empty() {
        "'none'".to_owned()
    } else {
        styles
            .iter()
            .map(|source| format!("'sha256-{}'", content_hash(source)))
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!(
        "default-src 'none'; script-src {script_source}; style-src {style_source}; img-src data:; font-src data:; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; worker-src 'none'"
    )
}

pub(super) fn theme_token_style_content(theme_tokens: &ThemeTokens) -> Option<String> {
    let css = render_theme_token_css(theme_tokens);
    (!css.is_empty()).then(|| format!("\n{css}"))
}

fn content_hash(content: &str) -> String {
    STANDARD.encode(Sha256::digest(content.as_bytes()))
}

fn token_value_is_safe(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(is_safe_css_color_value)
}

#[must_use]
pub fn render_vue_wrapper(input: &Report) -> String {
    render_vue_wrapper_with_theme_tokens(input, &ThemeTokens::default())
}

#[must_use]
pub fn render_vue_wrapper_with_theme_tokens(input: &Report, theme_tokens: &ThemeTokens) -> String {
    let payload = serde_json::to_string_pretty(input)
        .unwrap_or_else(|_| "{}".to_owned())
        .replace("</", "<\\/");
    let token_style = render_theme_token_style_block(theme_tokens);
    format!(
        r#"<template>
  <AgentUiRenderer :input="input" />
</template>

<script setup lang="ts">
import AgentUiRenderer from "./agent-ui-renderer/AgentUiRenderer.vue";
import type {{ Report }} from "./agent-ui-renderer/types";

const input = {payload} satisfies Report;
</script>
{token_style}"#
    )
}

#[must_use]
pub fn render_vue_html_shell(input: &Report) -> String {
    render_vue_html_shell_with_theme_tokens_and_language(input, &ThemeTokens::default(), "en")
}

#[must_use]
pub fn render_vue_html_shell_with_theme_tokens(
    input: &Report,
    theme_tokens: &ThemeTokens,
) -> String {
    render_vue_html_shell_with_theme_tokens_and_language(input, theme_tokens, "en")
}

#[must_use]
pub fn render_vue_html_shell_with_theme_tokens_and_language(
    input: &Report,
    theme_tokens: &ThemeTokens,
    document_language: &str,
) -> String {
    let title = escape_html(input.title.as_deref().unwrap_or("Agent UI Report"));
    let payload = script_safe_json(input);
    let token_style_content = theme_token_style_content(theme_tokens);
    let token_style = token_style_content
        .as_deref()
        .map_or_else(String::new, |content| format!("\n<style>{content}</style>"));
    let mut styles = vec![RENDERER_CSS];
    styles.extend(token_style_content.as_deref());
    let csp = render_content_security_policy(Some(RENDERER_JS), &styles);
    format!(
        r#"<!doctype html>
<html lang="{document_language}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="{csp}">
<title>{title}</title>
<style>{css}</style>{token_style}
</head>
<body class="agent-ui-standalone" data-theme="{theme}">
<div id="agent-ui-root"></div>
<noscript><main class="agent-ui-render"><section class="card"><h1>{title}</h1><p class="empty">This preview uses the embedded Vue renderer and requires JavaScript. Use <code>render static-html</code> for a no-JS artifact.</p></section></main></noscript>
<script type="application/json" id="agent-ui-payload">{payload}</script>
<script>{js}</script>
</body>
</html>
"#,
        css = RENDERER_CSS,
        js = RENDERER_JS,
        theme = escape_html(input.theme.as_deref().unwrap_or("report-light")),
        document_language = escape_html(document_language),
    )
}
