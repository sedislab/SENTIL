//! `sentil init` is an interactive one-shot guided builder.

use std::io::IsTerminal;

use inquire::{InquireError, Select, Text};

use crate::cli::{Algo, Backend, Interval, Method, Semantics};
use crate::error::{CliError, Run};
use crate::output::{self, Out};

use super::{check, smc, synth};

pub fn run(out: &Out) -> Run {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    if out.no_input || !interactive {
        return Err(CliError::Input(
            "init is an interactive builder and needs a terminal".into(),
            Some("run a verb directly, for example: sentil check -f 'always (x > 0)' -t run.csv".into()),
        ));
    }

    let mode = prompt_select(
        "What do you want to do?",
        &[
            "check  offline robustness over a trace",
            "smc    estimate a probabilistic spec",
            "synth  synthesize a control input",
        ],
    )?;
    match mode.split_whitespace().next() {
        Some("smc") => guided_smc(out),
        Some("synth") => guided_synth(out),
        _ => guided_check(out),
    }
}

fn guided_check(out: &Out) -> Run {
    let formula = prompt_text("Formula", "always[0, 5] (x > 0)")?;
    let trace = prompt_text("Trace file (or - for stdin)", "run.csv")?;
    let semantics = if prompt_select("Semantics", &["dense", "discrete"])? == "discrete" {
        Semantics::Discrete
    } else {
        Semantics::Dense
    };
    announce(
        out,
        &format!("sentil check -f '{formula}' -t {trace} --semantics {semantics}"),
    );
    check::run(
        Some(&formula),
        None,
        None,
        &[],
        &trace,
        semantics,
        false,
        Backend::Cpu,
        out,
    )
}

fn guided_smc(out: &Out) -> Run {
    let formula = prompt_text("Probabilistic formula", "P>=0.95(always[0, 10] (x > 0))")?;
    let trace = prompt_text("Base trace file", "base.csv")?;
    let algo = match prompt_select("Algorithm", &["smc", "sprt", "chernoff"])?.as_str() {
        "sprt" => Algo::Sprt,
        "chernoff" => Algo::Chernoff,
        _ => Algo::Smc,
    };
    let samples = prompt_text("Sample budget", "10000")?;
    announce(
        out,
        &format!("sentil smc -f '{formula}' -t {trace} --algo {algo} --samples {samples}"),
    );
    smc::run(
        algo, &samples, 0.95, Interval::Wilson, 0.05, 0.05, 42, Some(&formula), None, None, &[], &[],
        &trace, out,
    )
}

fn guided_synth(out: &Out) -> Run {
    let formula = prompt_text("Spec to satisfy", "always (x > 0)")?;
    let model = prompt_text("Model file", "system.json")?;
    let method = match prompt_select("Method", &["gradient", "cmaes", "milp"])?.as_str() {
        "cmaes" => Method::CmaEs,
        "milp" => Method::Milp,
        _ => Method::Gradient,
    };
    announce(
        out,
        &format!("sentil synth -f '{formula}' --model {model} --method {method}"),
    );
    synth::run(method, &model, Some(&formula), None, None, &[], None, 200, out)
}

fn announce(out: &Out, command: &str) {
    eprintln!("{}", out.paint(&format!("running: {command}"), output::dim()));
}

fn prompt_text(message: &str, default: &str) -> Result<String, CliError> {
    Text::new(message)
        .with_default(default)
        .prompt()
        .map_err(cancel_or_error)
}

fn prompt_select(message: &str, options: &[&str]) -> Result<String, CliError> {
    Select::new(message, options.to_vec())
        .prompt()
        .map(str::to_string)
        .map_err(cancel_or_error)
}

fn cancel_or_error(error: InquireError) -> CliError {
    match error {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            CliError::Input("cancelled".into(), None)
        }
        other => CliError::Input(format!("prompt failed: {other}"), None),
    }
}