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
pub use options::{LimitOverrides, Limits, RuntimeConfig, ValidationOptions};
pub use render::{render_static_html, render_vue_html_shell};
pub use spec::plan_ui_spec;
pub use validate::{
    validate_normalized_report, validate_normalized_report_with_options, validate_report,
    validate_report_with_options,
};
