use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "agent-ui-render",
    version,
    about = "Zero-install Agent UI renderer"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// Machine-readable or human-readable CLI output
    #[arg(short, long, value_enum, global = true, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

    /// Treat warnings as errors
    #[arg(long, global = true)]
    pub warnings_as_errors: bool,

    /// Suppress success messages
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Pretty-print JSON command outputs
    #[arg(long, global = true)]
    pub pretty: bool,

    /// Explicit JSON config path for validation/render limits and theme tokens
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Validate an Agent UI input payload
    Validate(InputCommand),
    /// Normalize compact input to schema=ui.input.normalized, version=1
    Normalize(IoCommand),
    /// Plan compact input into schema=ui.spec, version=1
    Plan(IoCommand),
    /// Render previews and handoff artifacts
    Render(RenderCommand),
    /// Print bundled JSON Schemas
    Schema(SchemaCommand),
    /// Generate shell completions
    Completion { shell: Shell },
}

#[derive(Debug, Args)]
pub struct InputCommand {
    /// Input JSON path, or '-' for stdin
    pub input: String,
}

#[derive(Debug, Args)]
pub struct IoCommand {
    /// Input JSON path, or '-' for stdin
    pub input: String,
    /// Output path; omitted writes JSON to stdout
    pub output_path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RenderCommand {
    #[command(subcommand)]
    pub target: RenderTarget,
}

#[derive(Debug, Subcommand)]
pub enum RenderTarget {
    /// Write self-contained browser HTML with embedded Vue client renderer
    Html(RenderFileCommand),
    /// Write no-JS static HTML fallback
    StaticHtml(RenderFileCommand),
    /// Write Vue SFC wrapper plus agent-ui-renderer/ handoff bundle
    Vue(RenderFileCommand),
}

#[derive(Debug, Args)]
pub struct RenderFileCommand {
    /// Input JSON path, or '-' for stdin
    pub input: String,
    /// Output artifact path
    pub output_path: PathBuf,
}

#[derive(Debug, Args)]
pub struct SchemaCommand {
    #[command(subcommand)]
    pub action: SchemaAction,
}

#[derive(Debug, Subcommand)]
pub enum SchemaAction {
    /// Print one bundled schema
    Print {
        #[arg(value_enum)]
        schema: SchemaName,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SchemaName {
    Compact,
    Normalized,
    Spec,
    Config,
}
