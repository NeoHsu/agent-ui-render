use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use agent_ui_render_core::{
    Finding, FindingLevel, Report, RuntimeConfig, ValidationOptions, ValidationReport,
    normalize_report, plan_ui_spec,
    render::{
        render_static_html_with_theme_tokens as render_static_html_document,
        render_vue_html_shell_with_theme_tokens, render_vue_wrapper_with_theme_tokens,
        vue_handoff_files,
    },
    validate_normalized_report_with_options, validate_report_with_options,
};
use anyhow::Context;
use serde_json::Value;

use crate::{
    cli::{
        GlobalArgs, InputCommand, IoCommand, OutputFormat, RenderFileCommand, SchemaAction,
        SchemaCommand, SchemaName,
    },
    error::{EXIT_RUNTIME, EXIT_WARNINGS_AS_ERRORS},
    output::{print_extra_warnings, print_findings, print_json, print_validation_result},
};

const COMPACT_SCHEMA: &str = include_str!("../../../../schemas/v1/compact.schema.json");
const COMPACT_V2_SCHEMA: &str = include_str!("../../../../schemas/v2/compact.schema.json");
const NORMALIZED_SCHEMA: &str = include_str!("../../../../schemas/v1/normalized.schema.json");
const NORMALIZED_V2_SCHEMA: &str = include_str!("../../../../schemas/v2/normalized.schema.json");
const SPEC_SCHEMA: &str = include_str!("../../../../schemas/v1/spec.schema.json");
const SPEC_V2_SCHEMA: &str = include_str!("../../../../schemas/v2/spec.schema.json");
const CONFIG_SCHEMA: &str = include_str!("../../../../schemas/config.schema.json");

pub fn validate(command: &InputCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    let options = validation_options(global)?;
    let payload = read_json(&command.input, options.limits.max_input_bytes)?;
    let report = validate_report_with_options(&payload, &options);
    print_validation_result(&report, global.output);
    exit_if_findings_block(&report, global);
    if !global.quiet && global.output == OutputFormat::Human {
        eprintln!("OK: payload is valid");
    }
    Ok(())
}

pub fn normalize(command: &IoCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    let options = validation_options(global)?;
    let payload = read_json(&command.input, options.limits.max_input_bytes)?;
    let normalized = validated_normalized_payload(&payload, global, &options)?;
    write_json_or_stdout(command.output_path.as_deref(), &normalized, global.pretty)?;
    if let (false, Some(output_path)) = (global.quiet, &command.output_path) {
        eprintln!("OK: wrote normalized report to {}", output_path.display());
    }
    Ok(())
}

pub fn plan(command: &IoCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    let options = validation_options(global)?;
    let payload = read_json(&command.input, options.limits.max_input_bytes)?;
    let normalized = validated_normalized_payload(&payload, global, &options)?;
    let spec = plan_ui_spec(&normalized);
    write_json_or_stdout(command.output_path.as_deref(), &spec, global.pretty)?;
    if let (false, Some(output_path)) = (global.quiet, &command.output_path) {
        eprintln!("OK: wrote UI spec to {}", output_path.display());
    }
    Ok(())
}

pub fn render_html(command: &RenderFileCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    let config = runtime_config(global)?;
    let options = config
        .clone()
        .apply_to_options(ValidationOptions::default());
    let payload = read_json(&command.input, options.limits.max_input_bytes)?;
    let normalized = validated_normalized_payload(&payload, global, &options)?;
    let html = render_vue_html_shell_with_theme_tokens(&normalized, &config.theme_tokens);
    warn_if_large_output(&command.output_path, &html, global, &options);
    write_text_file(&command.output_path, &html)?;
    if !global.quiet {
        eprintln!(
            "OK: wrote Vue client HTML to {}",
            command.output_path.display()
        );
    }
    Ok(())
}

pub fn render_static_html(command: &RenderFileCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    let config = runtime_config(global)?;
    let options = config
        .clone()
        .apply_to_options(ValidationOptions::default());
    let payload = read_json(&command.input, options.limits.max_input_bytes)?;
    let normalized = validated_normalized_payload(&payload, global, &options)?;
    let html = render_static_html_document(&normalized, &config.theme_tokens);
    warn_if_large_output(&command.output_path, &html, global, &options);
    write_text_file(&command.output_path, &html)?;
    if !global.quiet {
        eprintln!("OK: wrote static HTML to {}", command.output_path.display());
    }
    Ok(())
}

pub fn render_vue(command: &RenderFileCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    let config = runtime_config(global)?;
    let options = config
        .clone()
        .apply_to_options(ValidationOptions::default());
    let payload = read_json(&command.input, options.limits.max_input_bytes)?;
    let normalized = validated_normalized_payload(&payload, global, &options)?;
    let output_dir = command
        .output_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let renderer_dir = output_dir.join("agent-ui-renderer");
    if renderer_dir.exists() {
        fs::remove_dir_all(&renderer_dir)
            .with_context(|| format!("failed to remove {}", renderer_dir.display()))?;
    }
    for (relative, content) in vue_handoff_files() {
        let path = renderer_dir.join(relative);
        write_text_file(&path, content)?;
    }
    write_text_file(
        &command.output_path,
        &render_vue_wrapper_with_theme_tokens(&normalized, &config.theme_tokens),
    )?;
    if !global.quiet {
        eprintln!(
            "OK: wrote {} and {}",
            command.output_path.display(),
            renderer_dir.display()
        );
    }
    Ok(())
}

pub fn schema(command: &SchemaCommand, global: &GlobalArgs) -> anyhow::Result<()> {
    match command.action {
        SchemaAction::Print { schema } => {
            let source = match schema {
                SchemaName::Compact => COMPACT_SCHEMA,
                SchemaName::CompactV2 => COMPACT_V2_SCHEMA,
                SchemaName::Normalized => NORMALIZED_SCHEMA,
                SchemaName::NormalizedV2 => NORMALIZED_V2_SCHEMA,
                SchemaName::Spec => SPEC_SCHEMA,
                SchemaName::SpecV2 => SPEC_V2_SCHEMA,
                SchemaName::Config => CONFIG_SCHEMA,
            };
            if global.pretty {
                let value: Value = serde_json::from_str(source)?;
                print_json(&value, true)?;
            } else {
                println!("{source}");
            }
        }
    }
    Ok(())
}

fn validation_options(global: &GlobalArgs) -> anyhow::Result<ValidationOptions> {
    Ok(runtime_config(global)?.apply_to_options(ValidationOptions::default()))
}

fn runtime_config(global: &GlobalArgs) -> anyhow::Result<RuntimeConfig> {
    let Some(path) = &global.config else {
        return Ok(RuntimeConfig::default());
    };
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let config: RuntimeConfig = serde_json::from_str(&source)
        .with_context(|| format!("failed to parse config {}", path.display()))?;
    config
        .validate()
        .with_context(|| format!("invalid config {}", path.display()))?;
    Ok(config)
}

fn warn_if_large_output(
    path: &Path,
    content: &str,
    global: &GlobalArgs,
    options: &ValidationOptions,
) {
    let max = options.limits.warn_output_html_bytes;
    let bytes = content.len();
    if bytes <= max {
        return;
    }
    let warning = Finding {
        level: FindingLevel::Warning,
        path: path.display().to_string(),
        message: format!("output size {bytes} bytes exceeds warning threshold {max}"),
    };
    print_extra_warnings(&[warning], global.output);
    if global.warnings_as_errors {
        std::process::exit(EXIT_WARNINGS_AS_ERRORS);
    }
}

fn validated_normalized_payload(
    payload: &Value,
    global: &GlobalArgs,
    options: &ValidationOptions,
) -> anyhow::Result<Report> {
    let initial = validate_report_with_options(payload, options);
    print_findings(&initial, global.output);
    exit_if_findings_block(&initial, global);

    let normalized = normalize_report(payload)?;
    print_extra_warnings(&normalized.warnings, global.output);
    if global.warnings_as_errors && !normalized.warnings.is_empty() {
        std::process::exit(EXIT_WARNINGS_AS_ERRORS);
    }

    let normalized_value = serde_json::to_value(&normalized.input)?;
    let normalized_validation = validate_normalized_report_with_options(&normalized_value, options);
    print_findings(&normalized_validation, global.output);
    exit_if_findings_block(&normalized_validation, global);

    Ok(normalized.input)
}

fn exit_if_findings_block(report: &ValidationReport, global: &GlobalArgs) {
    if !report.errors.is_empty() {
        std::process::exit(EXIT_RUNTIME);
    }
    if global.warnings_as_errors && !report.warnings.is_empty() {
        std::process::exit(EXIT_WARNINGS_AS_ERRORS);
    }
}

fn read_json(path: &str, max_input_bytes: usize) -> anyhow::Result<Value> {
    let source = if path == "-" {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .context("failed to read stdin")?;
        ensure_input_size(path, source.len(), max_input_bytes)?;
        source
    } else {
        let metadata = fs::metadata(path).with_context(|| format!("failed to stat {path}"))?;
        ensure_input_size(path, metadata.len() as usize, max_input_bytes)?;
        let source = fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
        ensure_input_size(path, source.len(), max_input_bytes)?;
        source
    };
    serde_json::from_str(&source).with_context(|| format!("failed to parse JSON from {path}"))
}

fn ensure_input_size(path: &str, bytes: usize, max_input_bytes: usize) -> anyhow::Result<()> {
    if bytes > max_input_bytes {
        anyhow::bail!(
            "input {path} is {bytes} bytes, exceeding configured maxInputBytes {max_input_bytes}"
        );
    }
    Ok(())
}

fn write_json_or_stdout<T: serde::Serialize>(
    path: Option<&Path>,
    value: &T,
    pretty: bool,
) -> anyhow::Result<()> {
    let output = if pretty {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    } + "\n";
    if let Some(path) = path {
        write_text_file(path, &output)
    } else {
        print!("{output}");
        Ok(())
    }
}

fn write_text_file(path: &Path, content: &str) -> anyhow::Result<()> {
    ensure_parent_dir(path)?;
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}
