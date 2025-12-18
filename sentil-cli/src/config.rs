//! Layered configuration. This is so that one can set a config and share it with someone else.
//!
//! The precedence is a command-line flag, then a `SENTIL_*` environment variable, then `./sentil.toml`, then the user config under XDG, then
//! `/etc/sentil/config.toml`, then the built-in default.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use figment::providers::{Format, Toml};
use figment::Figment;
use serde::Deserialize;

use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

/// The settings a config file may carry.
#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    pub output: Option<String>,
    pub color: Option<String>,
    /// Command presets
    #[serde(default)]
    pub alias: HashMap<String, AliasValue>,
}

/// An alias body: `check = "check --spec controls/overshoot"`, or the same as a list of arguments.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AliasValue {
    Line(String),
    Args(Vec<String>),
}

impl AliasValue {
    /// The expansion as a token list
    pub fn tokens(&self) -> Vec<String> {
        match self {
            Self::Line(line) => shlex::split(line)
                .unwrap_or_else(|| line.split_whitespace().map(str::to_string).collect()),
            Self::Args(args) => args.clone(),
        }
    }
}

/// The files consulted with the lowest precedence first
pub fn search_paths(explicit: Option<&str>) -> Vec<PathBuf> {
    if let Some(path) = explicit {
        return vec![PathBuf::from(path)];
    }
    let mut paths = vec![PathBuf::from("/etc/sentil/config.toml")];
    if let Some(dirs) = ProjectDirs::from("io.github", "sedislab", "sentil") {
        paths.push(dirs.config_dir().join("config.toml"));
    }
    paths.push(PathBuf::from("sentil.toml"));
    paths
}

/// Reads and merges the config files that exist.
pub fn load(explicit: Option<&str>) -> Result<FileConfig, CliError> {
    if let Some(named) = explicit {
        if !Path::new(named).exists() {
            return Err(CliError::Input(
                format!("--config '{named}' does not exist"),
                Some("give the path to a TOML file, or leave --config off to use ./sentil.toml".into()),
            ));
        }
    }
    let mut figment = Figment::new();
    for path in search_paths(explicit) {
        if path.exists() {
            figment = figment.merge(Toml::file(path));
        }
    }
    figment.extract().map_err(config_error)
}

fn config_error(e: figment::Error) -> CliError {
    let file = e
        .metadata
        .as_ref()
        .and_then(|m| m.source.as_ref())
        .and_then(|s| s.file_path())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "the config file".into());
    CliError::Input(
        format!("{file}: {e}"),
        Some("the file takes output = \"json\", color = \"never\", and presets under [alias]".into()),
    )
}

/// `sentil config` where you define which files are consulted, which exist, and the values in effect.
pub fn show(explicit: Option<&str>, out: &Out) -> Run {
    println!("{}", out.paint("config files", output::heading()));
    for path in search_paths(explicit) {
        let mark = if path.exists() { "found  " } else { "absent " };
        println!("  {} {}", out.paint(mark, output::dim()), path.display());
    }
    let config = load(explicit)?;
    println!("\n{}", out.paint("values in effect", output::heading()));
    println!("  output  {}", config.output.as_deref().unwrap_or("text (default)"));
    println!("  color   {}", config.color.as_deref().unwrap_or("auto (default)"));

    if !config.alias.is_empty() {
        println!("\n{}", out.paint("aliases", output::heading()));
        let mut names: Vec<&String> = config.alias.keys().collect();
        names.sort();
        for name in names {
            println!("  {name} = {}", config.alias[name].tokens().join(" "));
        }
    }

    println!(
        "\n{}",
        out.paint(
            "a flag or a SENTIL_* environment variable overrides these.",
            output::dim()
        )
    );
    Ok(code::SUCCESS)
}