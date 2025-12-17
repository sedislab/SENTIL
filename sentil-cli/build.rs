use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use clap::CommandFactory;
use clap_complete::{generate_to, Shell};

#[path = "src/cli.rs"]
mod cli;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn main() {
    let hash = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let date = git(&["show", "-s", "--format=%cd", "--date=short", "HEAD"])
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=SENTIL_GIT_HASH={hash}");
    println!("cargo:rustc-env=SENTIL_COMMIT_DATE={date}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=src/cli.rs");

    let Some(outdir) = env::var_os("OUT_DIR") else {
        return;
    };
    let mut command = cli::Cli::command();
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
        let _ = generate_to(shell, &mut command, "sentil", &outdir);
    }
    let mut page = Vec::new();
    if clap_mangen::Man::new(command).render(&mut page).is_ok() {
        let _ = fs::write(PathBuf::from(&outdir).join("sentil.1"), page);
    }
}