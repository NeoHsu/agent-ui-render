pub mod chart;
pub mod diagnostic;
pub mod domain;
pub mod markdown;
pub mod normalize;
pub mod options;
pub mod render;
pub mod spec;
pub mod validate;
pub mod wire;

pub use diagnostic::{Finding, FindingLevel, ValidationReport};
pub use domain::{
    Alert, Column, Dataset, MarkdownSection, Metric, MetricDelta, Report, ViewIntent,
};
pub use normalize::{NormalizationResult, normalize_report};
pub use options::{
    DocumentLanguage, DocumentLanguageValidationError, LimitOverrides, Limits, RuntimeConfig,
    ThemeTokens, ValidationOptions, is_safe_css_color_value,
};
pub use render::{
    render_static_html, render_static_html_with_theme_tokens,
    render_static_html_with_theme_tokens_and_language, render_theme_token_css,
    render_vue_html_shell, render_vue_html_shell_with_theme_tokens,
    render_vue_html_shell_with_theme_tokens_and_language,
};
pub use spec::plan_ui_spec;
pub use validate::{
    validate_normalized_report, validate_normalized_report_with_options, validate_report,
    validate_report_with_options,
};
