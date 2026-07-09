use std::{fs, io, path::PathBuf, process::Command};

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
fn schema_print_outputs_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    for schema in ["compact", "normalized", "spec", "config"] {
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

fn workspace_root() -> Result<PathBuf, io::Error> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("workspace root should exist"))
}
