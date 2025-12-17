//! `sentil completion <shell>` and `sentil man`, emitting to stdout the same artifacts build.rs writes as files.

use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::Cli;
use crate::error::{code, CliError, Run};

pub fn completion(shell: Shell) -> Run {
    let mut command = Cli::command();
    clap_complete::generate(shell, &mut command, "sentil", &mut std::io::stdout());
    Ok(code::SUCCESS)
}

pub fn man() -> Run {
    let page = clap_mangen::Man::new(Cli::command());
    page.render(&mut std::io::stdout())
        .map_err(|e| CliError::Internal(format!("rendering the man page: {e}")))?;
    Ok(code::SUCCESS)
}