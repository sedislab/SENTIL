use crate::cli::Commands;
use crate::error::Run;
use crate::output::Out;

mod check;
mod specs;

/// Runs the chosen verb and returns its exit code.
pub fn dispatch(command: Commands, out: &Out) -> Run {
    match command {
        Commands::Check {
            formula,
            spec,
            variant,
            param,
            trace,
            semantics,
            backend,
        } => check::run(
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            &trace,
            semantics,
            backend,
            out,
        ),
        Commands::Specs { name, filter } => specs::run(name.as_deref(), filter.as_deref(), out),
    }
}