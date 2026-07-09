use serde_json::Value;

use crate::{
    chart::{chart_kind_for_view, column_index, extent, measure_keys, numeric_value},
    domain::{Alert, Column, Dataset, Metric, Primitive, Report, ViewIntent},
    markdown::markdown_to_html,
    options::{ThemeTokens, is_safe_css_color_value},
};

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

fn render_theme_token_style_block(theme_tokens: &ThemeTokens) -> String {
    let css = render_theme_token_css(theme_tokens);
    if css.is_empty() {
        String::new()
    } else {
        format!("\n<style>\n{css}</style>")
    }
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
    render_vue_html_shell_with_theme_tokens(input, &ThemeTokens::default())
}

#[must_use]
pub fn render_vue_html_shell_with_theme_tokens(
    input: &Report,
    theme_tokens: &ThemeTokens,
) -> String {
    let title = escape_html(input.title.as_deref().unwrap_or("Agent UI Report"));
    let payload = script_safe_json(input);
    let token_style = render_theme_token_style_block(theme_tokens);
    format!(
        r#"<!doctype html>
<html lang="zh-Hant">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
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
    )
}

#[must_use]
pub fn render_static_html(input: &Report) -> String {
    render_static_html_with_theme_tokens(input, &ThemeTokens::default())
}

#[must_use]
pub fn render_static_html_with_theme_tokens(input: &Report, theme_tokens: &ThemeTokens) -> String {
    let title = escape_html(input.title.as_deref().unwrap_or("Agent UI Report"));
    let token_style = render_theme_token_style_block(theme_tokens);
    let mut parts = vec![
        "<!doctype html>".to_owned(),
        "<html lang=\"zh-Hant\">".to_owned(),
        "<head>".to_owned(),
        "<meta charset=\"utf-8\">".to_owned(),
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">".to_owned(),
        format!("<title>{title}</title>"),
        format!("<style>{}</style>{token_style}", RENDERER_CSS),
        "</head>".to_owned(),
        format!(
            "<body class=\"agent-ui-standalone\" data-theme=\"{}\">",
            escape_html(input.theme.as_deref().unwrap_or("report-light"))
        ),
        format!(
            "<main class=\"agent-ui-render\" data-theme=\"{}\" data-density=\"{}\" data-emphasis=\"{}\">",
            escape_html(input.theme.as_deref().unwrap_or("report-light")),
            escape_html(input.density.as_deref().unwrap_or("comfortable")),
            escape_html(input.emphasis.as_deref().unwrap_or("strong"))
        ),
        "<header class=\"report-header\"><p class=\"eyebrow\">Structured report</p>".to_owned(),
        format!("<h1>{title}</h1>"),
    ];
    if let Some(summary) = &input.summary {
        parts.push(format!("<p class=\"summary\">{}</p>", escape_html(summary)));
    }
    parts.push("</header>".to_owned());
    parts.push(render_alerts(&input.alerts));
    parts.push(render_metrics(&input.metrics));
    parts.push(render_insights(&input.insights));
    for section in &input.markdown {
        parts.push(format!(
            "<section class=\"card markdown-card\">{}<div class=\"report-prose\">{}</div></section>",
            section.title.as_ref().map_or(String::new(), |title| format!("<h2>{}</h2>", escape_html(title))),
            markdown_to_html(&section.content)
        ));
    }
    parts.push(render_views(input));
    parts.push(render_assumptions(&input.assumptions));
    parts.push("<footer class=\"footer\">Structured report generated from validated input; payload text was escaped.</footer>".to_owned());
    parts.push("</main></body></html>".to_owned());
    parts
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn render_alerts(alerts: &[Alert]) -> String {
    if alerts.is_empty() {
        return String::new();
    }
    let items = alerts
        .iter()
        .map(|alert| {
            format!(
                "<article class=\"alert alert-{}\" role=\"{}\">{}<p>{}</p></article>",
                class_token(&alert.level),
                if matches!(alert.level.as_str(), "error" | "critical") {
                    "alert"
                } else {
                    "status"
                },
                alert.title.as_ref().map_or(String::new(), |title| format!(
                    "<strong>{}</strong>",
                    escape_html(title)
                )),
                escape_html(&alert.content)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("<section class=\"alerts\" aria-label=\"Alerts\">{items}</section>")
}

fn render_metrics(metrics: &[Metric]) -> String {
    if metrics.is_empty() {
        return String::new();
    }
    let items = metrics
        .iter()
        .map(|metric| {
            format!(
                "<article class=\"metric-card\"><div class=\"metric-label\">{}</div><div class=\"metric-value\">{}</div>{}</article>",
                escape_html(&metric.label),
                escape_html(&format_metric(metric)),
                metric.delta.as_ref().and_then(|delta| delta.label.as_ref()).map_or(String::new(), |label| format!("<div class=\"metric-delta\">{}</div>", escape_html(label)))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("<section class=\"metrics\" aria-label=\"Metrics\">{items}</section>")
}

fn render_insights(insights: &[String]) -> String {
    if insights.is_empty() {
        return String::new();
    }
    let items = insights
        .iter()
        .map(|item| format!("<li>{}</li>", escape_html(item)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<section class=\"card\"><h2>Key insights</h2><ul class=\"insights\">{items}</ul></section>"
    )
}

fn render_views(input: &Report) -> String {
    input
        .views
        .iter()
        .enumerate()
        .map(|(index, view)| {
            let title = escape_html(view.title.as_deref().unwrap_or(&view_title(view, index)));
            let body = input.datasets.get(&view.data).map_or_else(
                || "<p class=\"empty\">No dataset available for this view.</p>".to_owned(),
                |dataset| {
                    if view.intent == "precise_records" {
                        render_table_for_view(dataset, view)
                    } else {
                        render_chart_or_table(dataset, view)
                    }
                },
            );
            format!("<section class=\"card\"><h2>{title}</h2>{body}</section>")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_assumptions(assumptions: &[String]) -> String {
    if assumptions.is_empty() {
        return String::new();
    }
    let items = assumptions
        .iter()
        .map(|item| format!("<li>{}</li>", escape_html(item)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<section class=\"card muted\"><h2>Assumptions and limitations</h2><ul>{items}</ul></section>"
    )
}

fn render_chart_or_table(dataset: &Dataset, view: &ViewIntent) -> String {
    match chart_kind_for_view(view, dataset) {
        "line" => render_line_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
        "scatter" => render_scatter_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
        "pie" => render_pie_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
        _ => render_bar_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
    }
}

fn render_line_chart(dataset: &Dataset, view: &ViewIntent) -> Option<String> {
    let keys = measure_keys(dataset, view)
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();
    let mut all_values = Vec::new();
    for key in &keys {
        let index = column_index(dataset, Some(key))?;
        for row in &dataset.rows {
            if let Some(value) = numeric_value(row, index) {
                all_values.push(value);
            }
        }
    }
    if all_values.is_empty() {
        return None;
    }
    let (min_y, max_y) = extent(&all_values);
    let max_pos = (dataset.rows.len().saturating_sub(1)).max(1) as f64;
    let series = keys
        .iter()
        .enumerate()
        .filter_map(|(series_index, key)| {
            let index = column_index(dataset, Some(key))?;
            let points = dataset
                .rows
                .iter()
                .enumerate()
                .filter_map(|(row_index, row)| {
                    let value = numeric_value(row, index)?;
                    let x = 54.0 + (row_index as f64 / max_pos) * 682.0;
                    let y = 22.0 + (1.0 - (value - min_y) / (max_y - min_y)) * 204.0;
                    Some(format!("{x:.1},{y:.1}"))
                })
                .collect::<Vec<_>>()
                .join(" ");
            Some(format!(
                "<polyline fill=\"none\" stroke=\"var(--agent-series-{})\" stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\" points=\"{}\"/><text x=\"{}\" y=\"18\" fill=\"var(--agent-series-{})\" class=\"svg-label\">{}</text>",
                series_index + 1,
                points,
                54 + series_index * 170,
                series_index + 1,
                escape_html(&column_label(dataset, key))
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "<div class=\"chart\"><svg viewBox=\"0 0 760 280\" role=\"img\" aria-label=\"{}\"><rect x=\"54\" y=\"22\" width=\"682\" height=\"204\" rx=\"14\" fill=\"var(--agent-chart-bg)\" stroke=\"var(--agent-chart-border)\"/><line x1=\"54\" y1=\"226\" x2=\"736\" y2=\"226\" stroke=\"var(--agent-chart-axis)\"/><line x1=\"54\" y1=\"22\" x2=\"54\" y2=\"226\" stroke=\"var(--agent-chart-axis)\"/>{series}</svg></div>",
        escape_html(&chart_aria_label(view))
    ))
}

fn render_scatter_chart(dataset: &Dataset, view: &ViewIntent) -> Option<String> {
    let x_index = column_index(dataset, view.x.as_deref())?;
    let measure = measure_keys(dataset, view).into_iter().next()?;
    let y_index = column_index(dataset, Some(&measure))?;
    let points = dataset
        .rows
        .iter()
        .filter_map(|row| Some((numeric_value(row, x_index)?, numeric_value(row, y_index)?)))
        .collect::<Vec<_>>();
    if points.is_empty() {
        return None;
    }
    let (min_x, max_x) = extent(&points.iter().map(|point| point.0).collect::<Vec<_>>());
    let (min_y, max_y) = extent(&points.iter().map(|point| point.1).collect::<Vec<_>>());
    let circles = points
        .iter()
        .enumerate()
        .map(|(index, (x_value, y_value))| {
            let cx = 54.0 + ((*x_value - min_x) / (max_x - min_x)) * 682.0;
            let cy = 22.0 + (1.0 - (*y_value - min_y) / (max_y - min_y)) * 204.0;
            format!("<circle class=\"scatter-point\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"5\" fill=\"var(--agent-series-1)\" opacity=\"0.88\"><title>point {index}</title></circle>")
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "<div class=\"chart\"><svg viewBox=\"0 0 760 280\" role=\"img\" aria-label=\"{}\"><rect x=\"54\" y=\"22\" width=\"682\" height=\"204\" rx=\"14\" fill=\"var(--agent-chart-bg)\" stroke=\"var(--agent-chart-border)\"/><line x1=\"54\" y1=\"226\" x2=\"736\" y2=\"226\" stroke=\"var(--agent-chart-axis)\"/><line x1=\"54\" y1=\"22\" x2=\"54\" y2=\"226\" stroke=\"var(--agent-chart-axis)\"/>{circles}</svg></div>",
        escape_html(&chart_aria_label(view))
    ))
}

fn render_pie_chart(dataset: &Dataset, view: &ViewIntent) -> Option<String> {
    let x_index = column_index(dataset, view.x.as_deref())?;
    let measure = measure_keys(dataset, view).into_iter().next()?;
    let y_index = column_index(dataset, Some(&measure))?;
    let raw = dataset
        .rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let value = numeric_value(row, y_index).unwrap_or(0.0);
            (value > 0.0).then(|| (index, cell_plain(row.get(x_index)), value))
        })
        .collect::<Vec<_>>();
    let total: f64 = raw.iter().map(|item| item.2).sum();
    if total <= 0.0 {
        return None;
    }
    let circumference = 2.0 * std::f64::consts::PI * 92.0;
    let mut offset = 0.0;
    let slices = raw
        .iter()
        .enumerate()
        .map(|(slice_index, (_, label, value))| {
            let length = (*value / total) * circumference;
            let dash_offset = -offset;
            offset += length;
            format!("<circle cx=\"160\" cy=\"160\" r=\"92\" fill=\"none\" stroke=\"var(--agent-series-{})\" stroke-width=\"46\" stroke-dasharray=\"{length:.2} {circumference:.2}\" stroke-dashoffset=\"{dash_offset:.2}\"><title>{}</title></circle>", (slice_index % 6) + 1, escape_html(label))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let legend = raw
        .iter()
        .enumerate()
        .map(|(index, (_, label, value))| {
            format!("<div class=\"pie-legend-row\"><span class=\"pie-marker\" style=\"background: var(--agent-series-{})\"></span><span class=\"pie-label\">{}</span><span class=\"pie-value\">{}</span><span class=\"pie-percent\">{:.1}%</span></div>", (index % 6) + 1, escape_html(label), escape_html(&format_number(*value)), value / total * 100.0)
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "<div class=\"pie-chart\"><svg class=\"pie-svg\" viewBox=\"0 0 320 320\" role=\"img\" aria-label=\"{}\"><circle cx=\"160\" cy=\"160\" r=\"92\" fill=\"none\" stroke=\"var(--agent-border-soft)\" stroke-width=\"46\"/><g transform=\"rotate(-90 160 160)\">{slices}</g><text x=\"160\" y=\"154\" text-anchor=\"middle\" class=\"pie-total-label\">Total</text><text x=\"160\" y=\"180\" text-anchor=\"middle\" class=\"pie-total-value\">{}</text></svg><div class=\"pie-legend\" aria-label=\"Composition legend\">{legend}</div></div>",
        escape_html(&chart_aria_label(view)),
        escape_html(&format_number(total))
    ))
}

fn render_bar_chart(dataset: &Dataset, view: &ViewIntent) -> Option<String> {
    let x_index = column_index(dataset, view.x.as_deref()).unwrap_or(0);
    let measure = measure_keys(dataset, view).into_iter().next()?;
    let y_index = column_index(dataset, Some(&measure))?;
    let values = dataset
        .rows
        .iter()
        .filter_map(|row| numeric_value(row, y_index))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    let max = values.iter().copied().fold(1.0, f64::max);
    let y_column = dataset.columns.get(y_index);
    let rows = dataset
        .rows
        .iter()
        .map(|row| {
            let value = numeric_value(row, y_index).unwrap_or(0.0);
            let width = (value / max * 100.0).max(2.0);
            format!(
                "<div class=\"bar-row\"><div class=\"bar-label\">{}</div><div class=\"bar-track\"><div class=\"bar-fill\" style=\"width: {width:.1}%\"></div></div><div class=\"bar-value\">{}</div></div>",
                escape_html(&cell_plain(row.get(x_index))),
                escape_html(&format_cell_value(&Value::from(value), y_column))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("<div class=\"bar-chart\">{rows}</div>"))
}

fn render_table_for_view(dataset: &Dataset, view: &ViewIntent) -> String {
    let columns = selected_table_column_indexes(dataset, view);
    render_table_with_columns(dataset, &columns, "Dataset table")
}

fn selected_table_column_indexes(dataset: &Dataset, view: &ViewIntent) -> Vec<usize> {
    let keys = view
        .columns
        .as_ref()
        .filter(|columns| !columns.is_empty())
        .map_or_else(
            || {
                let mut fallback = Vec::new();
                if let Some(x) = &view.x {
                    fallback.push(x.as_str());
                }
                if let Some(dimensions) = &view.dimensions {
                    fallback.extend(dimensions.iter().map(String::as_str));
                }
                if let Some(measures) = &view.measures {
                    fallback.extend(measures.iter().map(String::as_str));
                }
                fallback
            },
            |columns| columns.iter().map(String::as_str).collect(),
        );

    let mut indexes = Vec::new();
    for key in keys {
        if let Some(index) = dataset.columns.iter().position(|column| column.key == key)
            && !indexes.contains(&index)
        {
            indexes.push(index);
        }
    }
    if indexes.is_empty() {
        (0..dataset.columns.len()).collect()
    } else {
        indexes
    }
}

fn render_table(dataset: &Dataset) -> String {
    let columns = (0..dataset.columns.len()).collect::<Vec<_>>();
    render_table_with_columns(dataset, &columns, "Dataset table")
}

fn render_table_with_columns(dataset: &Dataset, columns: &[usize], caption: &str) -> String {
    let headers = columns
        .iter()
        .filter_map(|index| dataset.columns.get(*index))
        .map(|column| {
            format!(
                "<th>{}</th>",
                escape_html(column.label.as_deref().unwrap_or(&column.key))
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let body = if dataset.rows.is_empty() {
        format!(
            "<tr><td colspan=\"{}\" class=\"empty\">No rows</td></tr>",
            columns.len().max(1)
        )
    } else {
        dataset
            .rows
            .iter()
            .map(|row| {
                let cells = columns
                    .iter()
                    .filter_map(|index| dataset.columns.get(*index).map(|column| (*index, column)))
                    .map(|(index, column)| {
                        format!(
                            "<td><span>{}</span></td>",
                            escape_html(&format_cell_value(
                                row.get(index).unwrap_or(&Value::Null),
                                Some(column)
                            ))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("");
                format!("<tr>{cells}</tr>")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "<div class=\"table-wrap\"><table><caption>{}</caption><thead><tr>{headers}</tr></thead><tbody>{body}</tbody></table></div>",
        escape_html(caption)
    )
}

fn format_metric(metric: &Metric) -> String {
    match metric.format.as_deref() {
        Some("currency") => format_currency(metric.value.as_f64(), metric.unit.as_deref()),
        Some("percent") => metric.value.as_f64().map_or_else(
            || cell_plain(Some(&metric.value)),
            |value| format!("{:.1}%", value * 100.0),
        ),
        Some("number") => metric
            .value
            .as_f64()
            .map_or_else(|| cell_plain(Some(&metric.value)), format_number),
        _ => cell_plain(Some(&metric.value)),
    }
}

fn format_cell_value(value: &Primitive, column: Option<&Column>) -> String {
    match column.and_then(|column| column.column_type.as_deref()) {
        Some("currency") => format_currency(
            value.as_f64(),
            column.and_then(|column| column.unit.as_deref()),
        ),
        Some("percent") => value.as_f64().map_or_else(
            || cell_plain(Some(value)),
            |number| format!("{:.1}%", number * 100.0),
        ),
        Some("number") => value
            .as_f64()
            .map_or_else(|| cell_plain(Some(value)), format_number),
        _ => cell_plain(Some(value)),
    }
}

fn format_currency(value: Option<f64>, unit: Option<&str>) -> String {
    value.map_or_else(
        || "—".to_owned(),
        |number| match unit {
            Some(unit) if !unit.is_empty() => format!("{unit} {}", format_number(number)),
            _ => format_number(number),
        },
    )
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

fn cell_plain(value: Option<&Value>) -> String {
    match value {
        Some(Value::Null) | None => "—".to_owned(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(other) => other.to_string(),
    }
}

fn column_label(dataset: &Dataset, key: &str) -> String {
    dataset
        .columns
        .iter()
        .find(|column| column.key == key)
        .and_then(|column| column.label.clone())
        .unwrap_or_else(|| key.to_owned())
}

fn chart_aria_label(view: &ViewIntent) -> String {
    format!("{} chart for {}", view.intent.replace('_', " "), view.data)
}

fn view_title(view: &ViewIntent, index: usize) -> String {
    match view.intent.as_str() {
        "precise_records" => "Records".to_owned(),
        "trend" => "Trend".to_owned(),
        "comparison" => "Comparison".to_owned(),
        "distribution" => "Distribution".to_owned(),
        "composition" => "Composition".to_owned(),
        "relationship" => "Relationship".to_owned(),
        "overview" => "Overview".to_owned(),
        _ => format!("View {}", index + 1),
    }
}

#[must_use]
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn script_safe_json(input: &Report) -> String {
    serde_json::to_string(input)
        .unwrap_or_else(|_| "{}".to_owned())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn class_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
