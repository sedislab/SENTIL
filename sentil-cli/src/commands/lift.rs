//! `sentil lift`: apply a spec's noise models to a trace, writing the lifted trace
//! as CSV to stdout so it pipes into `check` or `smc`.

use std::io::Write;

use crate::engine;
use crate::error::{code, CliError, Run};

pub fn run(
    spec: &str,
    variant: Option<&str>,
    params: &[String],
    trace_path: &str,
    seed: u64,
) -> Run {
    let builder = engine::resolve_builder(spec, variant, params)?;
    let lifting = builder
        .build_lifting_registry()
        .map_err(|e| CliError::Input(format!("the spec's noise models: {e}"), None))?;
    let trace = engine::load_trace(trace_path)?;
    let lifted = lifting
        .lift(&trace, seed)
        .map_err(|e| CliError::Engine(e.to_string()))?;

    let names = lifted.variables();
    let mut stdout = std::io::stdout();
    write_csv(&mut stdout, &lifted, &names)
        .map_err(|e| CliError::Internal(format!("writing the lifted trace: {e}")))?;
    Ok(code::SUCCESS)
}

fn write_csv(
    stdout: &mut std::io::Stdout,
    trace: &sentil::Trace,
    names: &[&str],
) -> std::io::Result<()> {
    writeln!(stdout, "time,{}", names.join(","))?;
    let times = trace.times();
    let columns: Vec<&[f64]> = names.iter().filter_map(|n| trace.signal(n)).collect();
    for (row, &time) in times.iter().enumerate() {
        let mut line = format!("{time}");
        for column in &columns {
            line.push(',');
            line.push_str(&column[row].to_string());
        }
        writeln!(stdout, "{line}")?;
    }
    Ok(())
}