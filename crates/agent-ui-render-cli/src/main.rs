mod cli;
mod commands;
mod error;
mod file_io;
mod output;

use clap::{CommandFactory, Parser};
use cli::{Cli, Command, RenderTarget};
use error::{classify_exit_code, exit_with, is_broken_pipe};
use output::write_stdout_bytes;

fn main() {
    let cli = Cli::parse();
    let output = cli.global.output;

    let result = match &cli.command {
        Command::Validate(command) => commands::validate(command, &cli.global),
        Command::Normalize(command) => commands::normalize(command, &cli.global),
        Command::Plan(command) => commands::plan(command, &cli.global),
        Command::Render(command) => match &command.target {
            RenderTarget::Html(command) => commands::render_html(command, &cli.global),
            RenderTarget::StaticHtml(command) => commands::render_static_html(command, &cli.global),
            RenderTarget::Vue(command) => commands::render_vue(command, &cli.global),
        },
        Command::Schema(command) => commands::schema(command, &cli.global),
        Command::Completion { shell } => {
            let mut command = Cli::command();
            let name = command.get_name().to_owned();
            let mut completion = Vec::new();
            clap_complete::generate(*shell, &mut command, name, &mut completion);
            write_stdout_bytes(&completion).map_err(Into::into)
        }
    };

    if let Err(error) = result {
        if is_broken_pipe(&error) {
            return;
        }
        let code = classify_exit_code(&error);
        exit_with(error, output, code);
    }
}
