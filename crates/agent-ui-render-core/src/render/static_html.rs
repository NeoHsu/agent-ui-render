use serde_json::Value;

use crate::{
    domain::{Alert, Dataset, Metric, Report, ViewIntent},
    markdown::markdown_to_html,
    options::ThemeTokens,
};

use super::{
    assets::{RENDERER_CSS, render_content_security_policy, theme_token_style_content},
    formatting::{
        class_token, escape_html, format_cell_value, format_metric, titleize_chart_name, view_title,
    },
    static_charts::render_chart_or_table,
};

#[must_use]
pub fn render_static_html(input: &Report) -> String {
    render_static_html_with_theme_tokens_and_language(input, &ThemeTokens::default(), "en")
}

#[must_use]
pub fn render_static_html_with_theme_tokens(input: &Report, theme_tokens: &ThemeTokens) -> String {
    render_static_html_with_theme_tokens_and_language(input, theme_tokens, "en")
}

#[must_use]
pub fn render_static_html_with_theme_tokens_and_language(
    input: &Report,
    theme_tokens: &ThemeTokens,
    document_language: &str,
) -> String {
    let title = escape_html(input.title.as_deref().unwrap_or("Agent UI Report"));
    let token_style_content = theme_token_style_content(theme_tokens);
    let token_style = token_style_content
        .as_deref()
        .map_or_else(String::new, |content| format!("\n<style>{content}</style>"));
    let mut styles = vec![RENDERER_CSS];
    styles.extend(token_style_content.as_deref());
    let csp = render_content_security_policy(None, &styles);
    let mut parts = vec![
        "<!doctype html>".to_owned(),
        format!("<html lang=\"{}\">", escape_html(document_language)),
        "<head>".to_owned(),
        "<meta charset=\"utf-8\">".to_owned(),
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">".to_owned(),
        format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{csp}\">"),
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
            let fallback_title = if view.intent == crate::domain::VIEW_INTENT_CHART {
                view.chart
                    .as_deref()
                    .map_or_else(|| "Chart".to_owned(), titleize_chart_name)
            } else {
                view_title(view, index)
            };
            let title = escape_html(view.title.as_deref().unwrap_or(&fallback_title));
            let body = input.datasets.get(&view.data).map_or_else(
                || "<p class=\"empty\">No dataset available for this view.</p>".to_owned(),
                |dataset| {
                    if view.intent == crate::domain::VIEW_INTENT_CHART {
                        format!(
                            "<p class=\"chart-static-note\">Interactive {} chart available in the JavaScript-enabled HTML output.</p>{}",
                            escape_html(view.chart.as_deref().unwrap_or("advanced")),
                            render_table(dataset)
                        )
                    } else if view.intent == "precise_records" {
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

pub(super) fn render_table(dataset: &Dataset) -> String {
    let columns = (0..dataset.columns.len()).collect::<Vec<_>>();
    render_table_with_columns(dataset, &columns, "Dataset table")
}

fn render_table_with_columns(dataset: &Dataset, columns: &[usize], caption: &str) -> String {
    let caption = escape_html(caption);
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
        "<div class=\"table-wrap\" role=\"region\" aria-label=\"{caption}\" tabindex=\"0\"><table><caption>{caption}</caption><thead><tr>{headers}</tr></thead><tbody>{body}</tbody></table></div>"
    )
}
