use anyhow::Error;
use serde_json::json;

use crate::cli::OutputFormat;

pub const EXIT_RUNTIME: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_WARNINGS_AS_ERRORS: i32 = 3;
pub const EXIT_IO: i32 = 4;

pub fn classify_exit_code(error: &Error) -> i32 {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
    {
        EXIT_IO
    } else {
        EXIT_RUNTIME
    }
}

pub fn exit_with(error: Error, output: OutputFormat, code: i32) -> ! {
    if output == OutputFormat::Json {
        let payload = json!({
            "error": {
                "kind": kind_for_code(code),
                "message": format!("{error:#}"),
                "status": code,
                "retryable": false
            }
        });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
        );
    } else {
        eprintln!("Error: {error:#}");
    }
    std::process::exit(code);
}

fn kind_for_code(code: i32) -> &'static str {
    match code {
        EXIT_USAGE => "usage",
        EXIT_WARNINGS_AS_ERRORS => "warning",
        EXIT_IO => "io",
        _ => "runtime",
    }
}
