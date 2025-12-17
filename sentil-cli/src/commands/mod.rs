use crate::cli::Commands;
use crate::error::Run;
use crate::output::Out;

mod check;
mod explain;
mod lift;
mod monitor;
mod smc;
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
        Commands::Monitor {
            formula,
            spec,
            variant,
            param,
        } => monitor::run(
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            out,
        ),
        Commands::Smc {
            algo,
            samples,
            confidence,
            interval,
            epsilon,
            seed,
            formula,
            spec,
            variant,
            param,
            trace,
        } => smc::run(
            algo,
            &samples,
            confidence,
            interval,
            epsilon,
            seed,
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            &trace,
            out,
        ),
        Commands::Lift {
            spec,
            variant,
            param,
            trace,
            seed,
        } => lift::run(&spec, variant.as_deref(), &param, &trace, seed),
        Commands::Explain { topic, fields } => explain::run(topic.as_deref(), fields, out),
        Commands::Specs { name, filter } => specs::run(name.as_deref(), filter.as_deref(), out),
    }
}