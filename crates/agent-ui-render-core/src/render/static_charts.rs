use serde_json::Value;

use crate::{
    chart::{
        bar_orientation_for_view, chart_kind_for_view, column_index, extent, measure_keys,
        numeric_value,
    },
    domain::{Dataset, ViewIntent},
};

use super::{
    formatting::{
        cell_plain, chart_aria_label, column_label, escape_html, format_cell_value, format_number,
    },
    static_html::render_table,
};

pub(super) fn render_chart_or_table(dataset: &Dataset, view: &ViewIntent) -> String {
    match chart_kind_for_view(view, dataset) {
        "line" => render_line_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
        "scatter" => render_scatter_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
        "pie" => render_pie_chart(dataset, view).unwrap_or_else(|| render_table(dataset)),
        _ if bar_orientation_for_view(view, dataset) == "vertical" => {
            render_vertical_bar_chart(dataset, view).unwrap_or_else(|| render_table(dataset))
        }
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
    let keys = measure_keys(dataset, view)
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return None;
    }
    let values = numeric_values_for_keys(dataset, &keys);
    if values.is_empty() {
        return None;
    }

    let global_max = values.into_iter().fold(1.0, f64::max);
    let shared_scale = measures_share_scale(dataset, &keys);
    let maxima = keys
        .iter()
        .map(|key| {
            if shared_scale {
                return global_max;
            }
            let index = column_index(dataset, Some(key)).unwrap_or(0);
            dataset
                .rows
                .iter()
                .filter_map(|row| numeric_value(row, index))
                .fold(1.0, f64::max)
        })
        .collect::<Vec<_>>();
    let legend = render_bar_legend(dataset, &keys);
    let groups = dataset
        .rows
        .iter()
        .map(|row| {
            let series = keys
                .iter()
                .enumerate()
                .filter_map(|(series_index, key)| {
                    let index = column_index(dataset, Some(key))?;
                    let column = dataset.columns.get(index);
                    let value = numeric_value(row, index).unwrap_or(0.0).max(0.0);
                    let width = (value / maxima[series_index] * 100.0).max(2.0);
                    let rendered_value = format_cell_value(&Value::from(value), column);
                    let fits_inside = can_fit_bar_label(width, &rendered_value);
                    let inside_value = fits_inside.then(|| {
                        format!(
                            "<span class=\"bar-value-inside\">{}</span>",
                            escape_html(&rendered_value)
                        )
                    });
                    let outside_value = if fits_inside {
                        "<div class=\"bar-value bar-value-placeholder\" aria-hidden=\"true\"></div>".to_owned()
                    } else {
                        format!(
                            "<div class=\"bar-value\">{}</div>",
                            escape_html(&rendered_value)
                        )
                    };
                    let series_name = (keys.len() > 1).then(|| {
                        format!(
                            "<div class=\"bar-series-name\">{}</div>",
                            escape_html(&column_label(dataset, key))
                        )
                    });
                    Some(format!(
                        "<div class=\"bar-series-row\">{}<div class=\"bar-track\"><div class=\"bar-fill\" style=\"width: {width:.1}%; background: var(--agent-series-{})\">{}</div></div>{outside_value}</div>",
                        series_name.unwrap_or_default(),
                        series_index + 1,
                        inside_value.unwrap_or_default()
                    ))
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "<div class=\"bar-group\"><div class=\"bar-label\">{}</div><div class=\"bar-series-list\">{series}</div></div>",
                escape_html(&cell_plain(row.get(x_index)))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let first_column = keys
        .first()
        .and_then(|key| column_index(dataset, Some(key)))
        .and_then(|index| dataset.columns.get(index));
    let axis_start = format_cell_value(&Value::from(0.0), first_column);
    let axis_end = if shared_scale {
        format_cell_value(&Value::from(global_max), first_column)
    } else {
        "Per-series scale".to_owned()
    };
    Some(format!(
        "<div class=\"bar-chart\" data-series-count=\"{}\">{legend}{groups}<div class=\"bar-axis\" data-shared=\"{shared_scale}\"><span>{}</span><span>{}</span></div></div>",
        keys.len(),
        escape_html(&axis_start),
        escape_html(&axis_end)
    ))
}

fn render_vertical_bar_chart(dataset: &Dataset, view: &ViewIntent) -> Option<String> {
    let x_index = column_index(dataset, view.x.as_deref())?;
    let keys = measure_keys(dataset, view)
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();
    if keys.is_empty() || dataset.rows.is_empty() {
        return None;
    }
    let maximum = numeric_values_for_keys(dataset, &keys)
        .into_iter()
        .map(|value| value.max(0.0))
        .fold(0.0, f64::max);
    if maximum <= 0.0 {
        return None;
    }

    let chart_maximum = maximum * 1.18;
    let group_step = 644.0 / dataset.rows.len() as f64;
    let cluster_width = (group_step * 0.7).min(118.0);
    let gap = if keys.len() > 1 { 4.0 } else { 0.0 };
    let bar_width = (cluster_width - gap * (keys.len() - 1) as f64) / keys.len() as f64;
    let value_column = keys
        .first()
        .and_then(|key| column_index(dataset, Some(key)))
        .and_then(|index| dataset.columns.get(index));
    let legend = render_bar_legend(dataset, &keys);
    let ticks = (0..5)
        .map(|index| {
            let ratio = index as f64 / 4.0;
            let value = ratio * chart_maximum;
            let y = 32.0 + (1.0 - ratio) * 190.0;
            format!(
                "<line x1=\"72\" y1=\"{y:.1}\" x2=\"716\" y2=\"{y:.1}\" class=\"chart-grid-line\"/><text x=\"62\" y=\"{:.1}\" text-anchor=\"end\" class=\"chart-axis-label\">{}</text>",
                y + 4.0,
                escape_html(&format_cell_value(&Value::from(value), value_column))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let groups = dataset
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let center = 72.0 + group_step * (row_index as f64 + 0.5);
            let start = center - cluster_width / 2.0;
            let bars = keys
                .iter()
                .enumerate()
                .filter_map(|(series_index, key)| {
                    let value_index = column_index(dataset, Some(key))?;
                    let column = dataset.columns.get(value_index);
                    let value = numeric_value(row, value_index).unwrap_or(0.0).max(0.0);
                    let height = value / chart_maximum * 190.0;
                    let y = 222.0 - height;
                    let x = start + series_index as f64 * (bar_width + gap);
                    let rendered_value = format_cell_value(&Value::from(value), column);
                    Some(format!(
                        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_width:.1}\" height=\"{height:.1}\" rx=\"3\" fill=\"var(--agent-series-{})\" class=\"vertical-bar\"><title>{}: {}</title></rect><text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" class=\"vertical-bar-value\">{}</text>",
                        series_index + 1,
                        escape_html(&column_label(dataset, key)),
                        escape_html(&rendered_value),
                        x + bar_width / 2.0,
                        (y - 6.0).max(41.0),
                        escape_html(&rendered_value)
                    ))
                })
                .collect::<Vec<_>>()
                .join("\n");
            let category = format_cell_value(row.get(x_index).unwrap_or(&Value::Null), dataset.columns.get(x_index));
            format!(
                "{bars}<text x=\"{center:.1}\" y=\"246\" text-anchor=\"middle\" class=\"chart-axis-label vertical-bar-category\">{}</text>",
                escape_html(&category)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "<div class=\"chart vertical-bar-chart\">{legend}<svg viewBox=\"0 0 760 270\" role=\"img\" aria-label=\"{}\">{ticks}<line x1=\"72\" y1=\"32\" x2=\"72\" y2=\"222\" class=\"chart-axis-line\"/><line x1=\"72\" y1=\"222\" x2=\"716\" y2=\"222\" class=\"chart-axis-line\"/>{groups}</svg></div>",
        escape_html(&chart_aria_label(view))
    ))
}

fn numeric_values_for_keys(dataset: &Dataset, keys: &[String]) -> Vec<f64> {
    keys.iter()
        .filter_map(|key| column_index(dataset, Some(key)))
        .flat_map(|index| {
            dataset
                .rows
                .iter()
                .filter_map(move |row| numeric_value(row, index))
        })
        .collect()
}

fn measures_share_scale(dataset: &Dataset, keys: &[String]) -> bool {
    let Some(first) = keys
        .first()
        .and_then(|key| column_index(dataset, Some(key)))
        .and_then(|index| dataset.columns.get(index))
    else {
        return false;
    };
    keys.iter().all(|key| {
        column_index(dataset, Some(key))
            .and_then(|index| dataset.columns.get(index))
            .is_some_and(|column| {
                column.column_type == first.column_type && column.unit == first.unit
            })
    })
}

fn render_bar_legend(dataset: &Dataset, keys: &[String]) -> String {
    if keys.len() <= 1 {
        return String::new();
    }
    let items = keys
        .iter()
        .enumerate()
        .map(|(index, key)| {
            format!(
                "<span class=\"chart-legend-item\"><span class=\"chart-legend-marker\" style=\"background: var(--agent-series-{})\"></span>{}</span>",
                index + 1,
                escape_html(&column_label(dataset, key))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("<div class=\"chart-legend\">{items}</div>")
}

fn can_fit_bar_label(width_percent: f64, label: &str) -> bool {
    width_percent >= (label.len() as f64 * 1.55).max(12.0)
}
