//! `sentil lift`: apply a spec's noise models to a trace, writing the lifted trace
//! as CSV to stdout so it pipes into `check` or `smc`.

use std::io::Write;

use crate::engine;
use crate::error::{code, CliError, Run};

pub fn run(
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    noise: &[String],
    trace_path: &str,
    seed: u64,
) -> Run {
    let lifting = if let Some(registry) = engine::parse_noise(noise)? {
        registry
    } else if let Some(name) = spec {
        engine::resolve_builder(name, variant, params)?
            .build_lifting_registry()
            .map_err(|e| CliError::Input(format!("the spec's noise models: {e}"), None))?
    } else {
        return Err(CliError::Input(
            "give a spec with --spec or a model with --noise".into(),
            Some("for example --noise 'speed=gaussian:0,0.5'".into()),
        ));
    };
    let trace = engine::load_trace(trace_path)?;
    let lifted = lifting
        .lift(&trace, seed)
        .map_err(|e| CliError::Engine(e.to_string()))?;

    let names = lifted.variables();
    let mut stdout = std::io::stdout();
    match write_csv(&mut stdout, &lifted, &names) {
        Ok(()) => Ok(code::SUCCESS),
        // A reader that closed the pipe early is a normal end, not an error.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(code::SUCCESS),
        Err(e) => Err(CliError::Internal(format!("writing the lifted trace: {e}"))),
    }
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