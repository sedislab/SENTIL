//! `sentil init`: a one-shot guided builder for a check. It only runs on a
//! terminal and never under `--no-input`, so a pipe or a CI job is never trapped in
//! a prompt. It prints the equivalent non-interactive command before running it, so
//! the wizard teaches the flags rather than hiding them.

use std::io::IsTerminal;

use inquire::{InquireError, Select, Text};

use crate::cli::{Backend, Semantics};
use crate::error::{CliError, Run};
use crate::output::{self, Out};

use super::check;

pub fn run(out: &Out) -> Run {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    if out.no_input || !interactive {
        return Err(CliError::Input(
            "init is an interactive builder and needs a terminal".into(),
            Some("run a verb directly, for example: sentil check -f 'always (x > 0)' -t run.csv".into()),
        ));
    }

    let formula = prompt_text("Formula to check:", "always[0, 5] (x > 0)")?;
    let trace = prompt_text("Trace file (or - for stdin):", "run.csv")?;
    let choice = prompt_select("Semantics:", &["dense", "discrete"])?;
    let semantics = if choice == "discrete" {
        Semantics::Discrete
    } else {
        Semantics::Dense
    };

    let command =
        format!("sentil check -f '{formula}' -t {trace} --semantics {semantics}");
    eprintln!("{}", out.paint(&format!("running: {command}"), output::dim()));

    check::run(
        Some(&formula),
        None,
        None,
        &[],
        &trace,
        semantics,
        false,
        Backend::Cpu,
        out,
    )
}

/// Runs a text prompt with a default, mapping a cancel to a clean error.
fn prompt_text(message: &str, default: &str) -> Result<String, CliError> {
    Text::new(message)
        .with_default(default)
        .prompt()
        .map_err(cancel_or_error)
}

fn prompt_select(message: &str, options: &[&str]) -> Result<String, CliError> {
    Select::new(message, options.to_vec())
        .prompt()
        .map(str::to_string)
        .map_err(cancel_or_error)
}

fn cancel_or_error(error: InquireError) -> CliError {
    match error {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            // The user backed out; surface it as input rather than a crash.
            CliError::Input("cancelled".into(), None)
        }
        other => CliError::Input(format!("prompt failed: {other}"), None),
    }
}