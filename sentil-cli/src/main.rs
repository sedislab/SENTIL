use clap::{CommandFactory, FromArgMatches};
use std::process::ExitCode;

mod cli;

const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\ncommit: ",
    env!("SENTIL_GIT_HASH"),
    "\ncommit date: ",
    env!("SENTIL_COMMIT_DATE"),
);

fn main() -> ExitCode {
    let command = cli::Cli::command().long_version(LONG_VERSION);
    let matches = command.get_matches();
    if cli::Cli::from_arg_matches(&matches).is_err() {
        return ExitCode::from(2);
    }
    // No verbs are wired yet; an invocation with nothing to do shows help.
    cli::Cli::command().print_help().ok();
    ExitCode::from(2)
}