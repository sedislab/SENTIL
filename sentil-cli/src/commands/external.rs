//! To allow you to write a subcommand that's not built in

use std::path::PathBuf;

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::config;
use crate::error::{code, CliError, Run};
use crate::output::Out;

pub fn run(args: &[String], config_path: Option<&str>, out: &Out) -> Run {
    let Some((name, rest)) = args.split_first() else {
        return Err(CliError::Input("no subcommand given".into(), None));
    };

    let aliases = config::load(config_path)?.alias;
    if let Some(alias) = aliases.get(name) {
        return run_alias(name, &alias.tokens(), rest, config_path, out);
    }

    if let Some(plugin) = find_plugin(name) {
        let status = std::process::Command::new(&plugin)
            .args(rest)
            .status()
            .map_err(|e| CliError::Internal(format!("running {}: {e}", plugin.display())))?;
        return Ok(status.code().and_then(|c| u8::try_from(c).ok()).unwrap_or(code::INTERNAL));
    }

    Err(CliError::Input(
        format!("unknown subcommand '{name}'"),
        Some("run `sentil --help` for the verbs, or define it under [alias] in sentil.toml".into()),
    ))
}

fn run_alias(
    name: &str,
    tokens: &[String],
    rest: &[String],
    config_path: Option<&str>,
    out: &Out,
) -> Run {
    let argv = std::iter::once("sentil".to_string())
        .chain(tokens.iter().cloned())
        .chain(rest.iter().cloned());
    let parsed = Cli::try_parse_from(argv).map_err(|e| {
        CliError::Input(
            format!("alias '{name}' does not expand to a valid command: {e}"),
            Some(format!("alias is: {}", tokens.join(" "))),
        )
    })?;
    match parsed.command {
        Some(Commands::External(_)) | None => Err(CliError::Input(
            format!("alias '{name}' must expand to a sentil verb"),
            None,
        )),
        Some(command) => super::dispatch(command, config_path, out),
    }
}

fn find_plugin(name: &str) -> Option<PathBuf> {
    let exe = format!("sentil-{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(&exe))
            .find(|candidate| is_executable(candidate))
    })
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &std::path::Path) -> bool {
    path.is_file()
}