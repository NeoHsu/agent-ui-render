use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
fn renders_html_from_example() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let output = temp.path().join("report.html");
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .args([
            "render",
            "html",
            input
                .to_str()
                .ok_or_else(|| io::Error::other("input path should be UTF-8"))?,
            output
                .to_str()
                .ok_or_else(|| io::Error::other("output path should be UTF-8"))?,
        ])
        .status()?;
    assert!(status.success());
    let html = fs::read_to_string(output)?;
    assert!(html.contains("agent-ui-root"));
    assert!(html.contains("Revenue Overview"));
    Ok(())
}

#[test]
fn render_vue_refuses_to_delete_unmanaged_directory() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let renderer_dir = temp.path().join("agent-ui-renderer");
    fs::create_dir(&renderer_dir)?;
    let sentinel = renderer_dir.join("custom.txt");
    fs::write(&sentinel, "keep me")?;
    let wrapper = temp.path().join("Report.vue");
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .args(["render", "vue"])
        .arg(input)
        .arg(&wrapper)
        .output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unmanaged directory"));
    assert_eq!(fs::read_to_string(sentinel)?, "keep me");
    assert!(!wrapper.exists());
    Ok(())
}

#[test]
fn render_vue_force_replaces_unmanaged_directory() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let renderer_dir = temp.path().join("agent-ui-renderer");
    fs::create_dir(&renderer_dir)?;
    fs::write(renderer_dir.join("custom.txt"), "replace me")?;
    let wrapper = temp.path().join("Report.vue");
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .args(["render", "vue"])
        .arg(&input)
        .arg(&wrapper)
        .arg("--force")
        .status()?;

    assert!(status.success());
    assert!(wrapper.is_file());
    assert!(renderer_dir.join("AgentUiRenderer.vue").is_file());
    assert!(renderer_dir.join(".agent-ui-render-managed").is_file());
    assert!(!renderer_dir.join("custom.txt").exists());

    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .args(["render", "vue"])
        .arg(&input)
        .arg(&wrapper)
        .status()?;
    assert!(
        status.success(),
        "managed handoff should update without --force"
    );
    Ok(())
}

#[test]
fn invalid_payload_exits_nonzero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("bad.json");
    fs::write(&input, r#"{"version":1,"t":"<script>bad()</script>"}"#)?;
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("validate")
        .arg(input)
        .status()?;
    assert!(!status.success());
    Ok(())
}

#[test]
fn valid_payload_json_output_is_one_document_on_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .args(["--output", "json", "validate"])
        .arg(input)
        .output()?;

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "expected empty stderr, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["valid"], true);
    assert_eq!(value["errors"], serde_json::json!([]));
    assert_eq!(value["warnings"], serde_json::json!([]));
    Ok(())
}

#[test]
fn invalid_payload_json_output_is_one_document_on_stdout() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("bad.json");
    fs::write(&input, r#"{"version":1,"t":"<script>bad()</script>"}"#)?;
    let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .args(["--output", "json", "validate"])
        .arg(input)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stderr.is_empty(),
        "expected empty stderr, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["valid"], false);
    let errors = value["errors"]
        .as_array()
        .ok_or("validation errors should be an array")?;
    assert!(!errors.is_empty());
    Ok(())
}

#[test]
fn config_limits_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(&config, r#"{"limits":{"maxRowsPerDataset":1}}"#)?;
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .arg("validate")
        .arg(input)
        .status()?;
    assert!(!status.success());
    Ok(())
}

#[test]
fn theme_tokens_config_is_embedded_in_rendered_html() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(
        &config,
        r##"{"themeTokens":{"page":"#0b1220","text":"#f9fafb","primary":"#8b5cf6","series1":"#06b6d4"}}"##,
    )?;
    let output = temp.path().join("report.static.html");
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .args(["render", "static-html"])
        .arg(input)
        .arg(&output)
        .status()?;
    assert!(status.success());

    let html = fs::read_to_string(output)?;
    assert!(html.contains("--agent-primary: #8b5cf6;"));
    assert!(html.contains("--agent-series-1: #06b6d4;"));
    assert!(html.contains("background: var(--agent-page);"));
    assert!(html.contains("color: var(--agent-text);"));
    Ok(())
}

#[test]
fn unsafe_theme_token_config_exits_nonzero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(
        &config,
        r##"{"themeTokens":{"primary":"#fff;}</style><script>bad()</script>"}}"##,
    )?;
    let output = temp.path().join("report.static.html");
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .args(["render", "static-html"])
        .arg(input)
        .arg(output)
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid theme token"));
    Ok(())
}

#[test]
fn max_input_bytes_is_enforced_before_parse() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(&config, r#"{"limits":{"maxInputBytes":10}}"#)?;
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .arg("validate")
        .arg(input)
        .status()?;
    assert!(!status.success());
    Ok(())
}

#[test]
fn max_input_bytes_bounds_stdin_reads() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(&config, r#"{"limits":{"maxInputBytes":10}}"#)?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .args(["validate", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin should be piped")?
        .write_all(br#"{"version":1,"t":"payload larger than ten bytes"}"#)?;
    let output = child.wait_with_output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("maxInputBytes 10"));
    Ok(())
}

#[test]
fn schema_print_outputs_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    for schema in [
        "compact",
        "compact-v2",
        "normalized",
        "normalized-v2",
        "spec",
        "spec-v2",
        "config",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
            .args(["schema", "print", schema])
            .output()?;
        assert!(
            output.status.success(),
            "schema print {schema} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }
    Ok(())
}

#[test]
fn output_html_size_warning_can_be_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(&config, r#"{"limits":{"warnOutputHtmlBytes":1}}"#)?;
    let output = temp.path().join("report.static.html");
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let status = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .arg("--warnings-as-errors")
        .args(["render", "static-html"])
        .arg(input)
        .arg(output)
        .status()?;
    assert_eq!(status.code(), Some(3));
    Ok(())
}

#[test]
fn hard_output_html_limit_preserves_existing_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("agent-ui-render.config.json");
    fs::write(&config, r#"{"limits":{"maxOutputHtmlBytes":1}}"#)?;
    let output_path = temp.path().join("report.static.html");
    fs::write(&output_path, "existing")?;
    let input = workspace_root()?.join("examples/revenue-overview.input.json");
    let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
        .arg("--config")
        .arg(config)
        .args(["render", "static-html"])
        .arg(input)
        .arg(&output_path)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("maxOutputHtmlBytes 1"));
    assert_eq!(fs::read_to_string(output_path)?, "existing");
    Ok(())
}

#[test]
fn help_describes_version_preserving_v1_and_v2_pipeline() -> Result<(), Box<dyn std::error::Error>>
{
    for (command, expected) in [
        (
            "normalize",
            "Normalize compact v1/v2 input to schema=ui.input.normalized while preserving its version",
        ),
        (
            "plan",
            "Plan compact v1/v2 input into schema=ui.spec while preserving its version",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agent-ui-render"))
            .args([command, "--help"])
            .output()?;
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout)?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(help.contains(expected), "unexpected {command} help: {help}");
        assert!(!help.contains("version=1"), "stale {command} help: {help}");
    }
    Ok(())
}

fn workspace_root() -> Result<PathBuf, io::Error> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("workspace root should exist"))
}
