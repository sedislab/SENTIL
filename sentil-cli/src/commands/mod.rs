use crate::cli::Commands;
use crate::config;
use crate::error::Run;
use crate::output::Out;

mod check;
mod explain;
mod external;
mod generate;
mod init;
mod lift;
mod mine;
mod monitor;
mod smc;
mod specs;
mod synth;

pub fn dispatch(command: Commands, config_path: Option<&str>, out: &Out) -> Run {
    match command {
        Commands::Init => init::run(out),
        Commands::Config => config::show(config_path, out),
        Commands::Check {
            formula,
            spec,
            variant,
            param,
            trace,
            map,
            semantics,
            signal,
            violations,
            backend,
        } => check::run(
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            &trace,
            &map,
            semantics,
            signal,
            violations,
            backend,
            out,
        ),
        Commands::Monitor {
            formula,
            spec,
            variant,
            param,
            map,
        } => monitor::run(
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            &map,
            out,
        ),
        Commands::Smc {
            algo,
            samples,
            confidence,
            interval,
            epsilon,
            indifference,
            seed,
            formula,
            spec,
            variant,
            param,
            noise,
            trace,
            map,
        } => smc::run(
            algo,
            &samples,
            confidence,
            interval,
            epsilon,
            indifference,
            seed,
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            &noise,
            &trace,
            &map,
            out,
        ),
        Commands::Synth {
            method,
            model,
            formula,
            spec,
            variant,
            param,
            horizon,
            budget,
        } => synth::run(
            method,
            &model,
            formula.as_deref(),
            spec.as_deref(),
            variant.as_deref(),
            &param,
            horizon,
            budget,
            out,
        ),
        Commands::Mine {
            spec,
            variant,
            param,
            parameter,
            range,
            trace,
            map,
        } => mine::run(
            &spec,
            variant.as_deref(),
            &param,
            &parameter,
            range.as_deref(),
            &trace,
            &map,
            out,
        ),
        Commands::Lift {
            spec,
            variant,
            param,
            noise,
            trace,
            map,
            members,
            seed,
        } => lift::run(
            spec.as_deref(),
            variant.as_deref(),
            &param,
            &noise,
            &trace,
            &map,
            members,
            seed,
        ),
        Commands::Completion { shell } => generate::completion(shell),
        Commands::Man => generate::man(),
        Commands::Explain { topic, fields } => explain::run(topic.as_deref(), fields, out),
        Commands::Specs { name, filter } => specs::run(name.as_deref(), filter.as_deref(), out),
        Commands::External(args) => external::run(&args, config_path, out),
    }
}