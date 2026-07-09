use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_datasets: usize,
    pub max_columns_per_dataset: usize,
    pub max_rows_per_dataset: usize,
    pub max_cells_per_dataset: usize,
    pub max_metrics: usize,
    pub max_views: usize,
    pub max_alerts: usize,
    pub max_markdown_sections: usize,
    pub max_string_chars: usize,
    pub max_markdown_section_chars: usize,
    pub max_total_markdown_chars: usize,
    pub warn_output_html_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 5 * 1024 * 1024,
            max_datasets: 20,
            max_columns_per_dataset: 50,
            max_rows_per_dataset: 2_000,
            max_cells_per_dataset: 100_000,
            max_metrics: 50,
            max_views: 50,
            max_alerts: 50,
            max_markdown_sections: 20,
            max_string_chars: 20_000,
            max_markdown_section_chars: 50_000,
            max_total_markdown_chars: 200_000,
            warn_output_html_bytes: 5 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationOptions {
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub limits: LimitOverrides,
}

impl RuntimeConfig {
    #[must_use]
    pub fn apply_to_options(self, mut options: ValidationOptions) -> ValidationOptions {
        self.limits.apply_to(&mut options.limits);
        options
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LimitOverrides {
    pub max_input_bytes: Option<usize>,
    pub max_datasets: Option<usize>,
    pub max_columns_per_dataset: Option<usize>,
    pub max_rows_per_dataset: Option<usize>,
    pub max_cells_per_dataset: Option<usize>,
    pub max_metrics: Option<usize>,
    pub max_views: Option<usize>,
    pub max_alerts: Option<usize>,
    pub max_markdown_sections: Option<usize>,
    pub max_string_chars: Option<usize>,
    pub max_markdown_section_chars: Option<usize>,
    pub max_total_markdown_chars: Option<usize>,
    pub warn_output_html_bytes: Option<usize>,
}

impl LimitOverrides {
    fn apply_to(self, limits: &mut Limits) {
        apply(self.max_input_bytes, &mut limits.max_input_bytes);
        apply(self.max_datasets, &mut limits.max_datasets);
        apply(
            self.max_columns_per_dataset,
            &mut limits.max_columns_per_dataset,
        );
        apply(self.max_rows_per_dataset, &mut limits.max_rows_per_dataset);
        apply(
            self.max_cells_per_dataset,
            &mut limits.max_cells_per_dataset,
        );
        apply(self.max_metrics, &mut limits.max_metrics);
        apply(self.max_views, &mut limits.max_views);
        apply(self.max_alerts, &mut limits.max_alerts);
        apply(
            self.max_markdown_sections,
            &mut limits.max_markdown_sections,
        );
        apply(self.max_string_chars, &mut limits.max_string_chars);
        apply(
            self.max_markdown_section_chars,
            &mut limits.max_markdown_section_chars,
        );
        apply(
            self.max_total_markdown_chars,
            &mut limits.max_total_markdown_chars,
        );
        apply(
            self.warn_output_html_bytes,
            &mut limits.warn_output_html_bytes,
        );
    }
}

fn apply(value: Option<usize>, target: &mut usize) {
    if let Some(value) = value {
        *target = value;
    }
}
