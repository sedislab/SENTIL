use clap::{CommandFactory, FromArgMatches};
use std::process::ExitCode;

mod cli;
mod commands;
mod engine;
mod error;
mod output;

const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\ncommit: ",
    env!("SENTIL_GIT_HASH"),
    "\ncommit date: ",
    env!("SENTIL_COMMIT_DATE"),
);

fn install_diagnostic_hook() {
    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(false)
                .unicode(true)
                .context_lines(2)
                .tab_width(4)
                .build(),
        )
    }));
}

fn run() -> error::Run {
    let command = cli::Cli::command().long_version(LONG_VERSION);
    let matches = command.get_matches();
    let cli =
        cli::Cli::from_arg_matches(&matches).map_err(|e| error::CliError::Internal(e.to_string()))?;
    let out = output::Out::new(cli.output, cli.json, cli.color, cli.quiet);
    match cli.command {
        Some(command) => commands::dispatch(command, &out),
        None => {
            // Help and a usage exit rather than a prompt, so a pipe or a CI job never hangs.
            cli::Cli::command().print_help().ok();
            Ok(error::code::USAGE)
        }
    }
}

#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() -> ExitCode {
    reset_sigpipe();
    install_diagnostic_hook();
    match run() {
        Ok(exit) => ExitCode::from(exit),
        Err(err) => {
            let exit = err.exit_code();
            eprintln!("{:?}", miette::Report::new(err));
            ExitCode::from(exit)
        }
    }
}