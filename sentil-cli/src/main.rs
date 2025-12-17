use clap::parser::ValueSource;
use clap::{ArgMatches, CommandFactory, FromArgMatches, ValueEnum};
use std::process::ExitCode;

mod cli;
mod commands;
mod config;
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

    let file = config::load(cli.config.as_deref())?;
    let output = from_config(&matches, "output", cli.output, file.output.as_deref());
    let color = from_config(&matches, "color", cli.color, file.color.as_deref());
    let out = output::Out::new(output, cli.json, color, cli.quiet);

    match cli.command {
        Some(command) => commands::dispatch(command, cli.config.as_deref(), &out),
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

// Lets a config-file value replace a flag's default, but only when the parser used
// that default (no flag and no env var). The file value is parsed against the same
// value enum the flag uses, so an invalid entry is ignored rather than fatal.
fn from_config<T: ValueEnum + Copy>(
    matches: &ArgMatches,
    name: &str,
    flag_value: T,
    file_value: Option<&str>,
) -> T {
    if matches.value_source(name) == Some(ValueSource::DefaultValue) {
        if let Some(text) = file_value {
            if let Ok(parsed) = T::from_str(text, true) {
                return parsed;
            }
        }
    }
    flag_value
}

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