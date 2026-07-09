use std::{error::Error, fmt};

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
    #[serde(default)]
    pub theme_tokens: ThemeTokens,
}

impl RuntimeConfig {
    #[must_use]
    pub fn apply_to_options(self, mut options: ValidationOptions) -> ValidationOptions {
        self.limits.apply_to(&mut options.limits);
        options
    }

    /// Validate trusted host configuration before any renderer CSS is emitted.
    ///
    /// Theme token values are intentionally limited to safe CSS color literals so
    /// a config file cannot accidentally smuggle extra CSS declarations into the
    /// generated HTML.
    pub fn validate(&self) -> Result<(), ThemeTokenValidationError> {
        self.theme_tokens.validate()
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

macro_rules! theme_tokens {
    ($( $field:ident => ($key:literal, $css_var:literal) ),+ $(,)?) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct ThemeTokens {
            $(
                #[serde(default, rename = $key, skip_serializing_if = "Option::is_none")]
                pub $field: Option<String>,
            )+
        }

        impl ThemeTokens {
            pub const KEYS: &'static [&'static str] = &[$($key),+];

            #[must_use]
            pub fn entries(&self) -> Vec<ThemeTokenEntry<'_>> {
                let mut entries = Vec::new();
                $(
                    if let Some(value) = &self.$field {
                        entries.push(ThemeTokenEntry {
                            key: $key,
                            css_var: $css_var,
                            value,
                        });
                    }
                )+
                entries
            }

            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.entries().is_empty()
            }

            pub fn validate(&self) -> Result<(), ThemeTokenValidationError> {
                let violations = self
                    .entries()
                    .into_iter()
                    .filter(|entry| !is_safe_css_color_value(entry.value))
                    .map(|entry| ThemeTokenViolation {
                        key: entry.key,
                        value: entry.value.to_owned(),
                    })
                    .collect::<Vec<_>>();

                if violations.is_empty() {
                    Ok(())
                } else {
                    Err(ThemeTokenValidationError { violations })
                }
            }
        }
    };
}

theme_tokens! {
    page => ("page", "--agent-page"),
    bg => ("bg", "--agent-bg"),
    surface => ("surface", "--agent-surface"),
    surface_muted => ("surfaceMuted", "--agent-surface-muted"),
    surface_strong => ("surfaceStrong", "--agent-surface-strong"),
    border => ("border", "--agent-border"),
    border_soft => ("borderSoft", "--agent-border-soft"),
    text => ("text", "--agent-text"),
    muted => ("muted", "--agent-muted"),
    subtle => ("subtle", "--agent-subtle"),
    primary => ("primary", "--agent-primary"),
    accent => ("accent", "--agent-accent"),
    info => ("info", "--agent-info"),
    success => ("success", "--agent-success"),
    error => ("error", "--agent-error"),
    code_bg => ("codeBg", "--agent-code-bg"),
    code_fg => ("codeFg", "--agent-code-fg"),
    code_border => ("codeBorder", "--agent-code-border"),
    pre_bg => ("preBg", "--agent-pre-bg"),
    pre_fg => ("preFg", "--agent-pre-fg"),
    pre_border => ("preBorder", "--agent-pre-border"),
    chart_bg => ("chartBg", "--agent-chart-bg"),
    chart_border => ("chartBorder", "--agent-chart-border"),
    chart_axis => ("chartAxis", "--agent-chart-axis"),
    series_1 => ("series1", "--agent-series-1"),
    series_2 => ("series2", "--agent-series-2"),
    series_3 => ("series3", "--agent-series-3"),
    series_4 => ("series4", "--agent-series-4"),
    series_5 => ("series5", "--agent-series-5"),
    series_6 => ("series6", "--agent-series-6"),
    critical_bg => ("criticalBg", "--agent-critical-bg"),
    critical_soft => ("criticalSoft", "--agent-critical-soft"),
    critical_border => ("criticalBorder", "--agent-critical-border"),
    critical_fg => ("criticalFg", "--agent-critical-fg"),
    error_bg => ("errorBg", "--agent-error-bg"),
    error_soft => ("errorSoft", "--agent-error-soft"),
    error_border => ("errorBorder", "--agent-error-border"),
    error_fg => ("errorFg", "--agent-error-fg"),
    warning_bg => ("warningBg", "--agent-warning-bg"),
    warning_border => ("warningBorder", "--agent-warning-border"),
    warning_fg => ("warningFg", "--agent-warning-fg"),
    success_bg => ("successBg", "--agent-success-bg"),
    success_border => ("successBorder", "--agent-success-border"),
    success_fg => ("successFg", "--agent-success-fg"),
    info_bg => ("infoBg", "--agent-info-bg"),
    info_border => ("infoBorder", "--agent-info-border"),
    info_fg => ("infoFg", "--agent-info-fg"),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeTokenEntry<'a> {
    pub key: &'static str,
    pub css_var: &'static str,
    pub value: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokenViolation {
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokenValidationError {
    violations: Vec<ThemeTokenViolation>,
}

impl ThemeTokenValidationError {
    #[must_use]
    pub fn violations(&self) -> &[ThemeTokenViolation] {
        &self.violations
    }
}

impl fmt::Display for ThemeTokenValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let details = self
            .violations
            .iter()
            .map(|violation| format!("{}={:?}", violation.key, violation.value))
            .collect::<Vec<_>>()
            .join(", ");
        write!(formatter, "invalid theme token color value(s): {details}")
    }
}

impl Error for ThemeTokenValidationError {}

#[must_use]
pub fn is_safe_css_color_value(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > 96 || value.chars().any(char::is_control) {
        return false;
    }

    if value == "transparent" || value == "currentColor" {
        return true;
    }

    is_hex_color(value) || is_color_function(value)
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|character| character.is_ascii_hexdigit())
}

fn is_color_function(value: &str) -> bool {
    let Some(open_paren) = value.find('(') else {
        return false;
    };
    if !value.ends_with(')') || value[..open_paren].contains(char::is_whitespace) {
        return false;
    }

    let name = &value[..open_paren];
    if !matches!(
        name,
        "rgb" | "rgba" | "hsl" | "hsla" | "hwb" | "lab" | "lch" | "oklab" | "oklch"
    ) {
        return false;
    }

    let args = &value[open_paren + 1..value.len() - 1];
    !args.is_empty()
        && args.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, ' ' | '\t' | '.' | ',' | '%' | '/' | '+' | '-')
        })
}

fn apply(value: Option<usize>, target: &mut usize) {
    if let Some(value) = value {
        *target = value;
    }
}
