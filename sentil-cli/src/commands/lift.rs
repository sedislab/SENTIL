//! `sentil lift` applies noise models to a trace

use std::io::Write;

use crate::engine;
use crate::error::{code, CliError, Run};

#[allow(clippy::too_many_arguments)]
pub fn run(
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    noise: &[String],
    trace_path: &str,
    map: &[String],
    members: u64,
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
    let trace = engine::load_trace(trace_path, map)?;
    let members = members.max(1);
    let tagged = members > 1;
    let names: Vec<String> = trace.variables().iter().map(|s| s.to_string()).collect();

    let mut stdout = std::io::stdout();
    let header = if tagged {
        format!("member,time,{}", names.join(","))
    } else {
        format!("time,{}", names.join(","))
    };
    if let Err(e) = writeln!(stdout, "{header}") {
        return pipe_or_error(e);
    }
    for member in 0..members {
        let lifted = lifting
            .lift(&trace, seed + member)
            .map_err(|e| CliError::Engine(e.to_string()))?;
        if let Err(e) = write_rows(&mut stdout, &lifted, &names, tagged.then_some(member)) {
            return pipe_or_error(e);
        }
    }
    Ok(code::SUCCESS)
}

fn pipe_or_error(e: std::io::Error) -> Run {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        Ok(code::SUCCESS)
    } else {
        Err(CliError::Internal(format!("writing the lifted trace: {e}")))
    }
}

fn write_rows(
    stdout: &mut std::io::Stdout,
    trace: &sentil::Trace,
    names: &[String],
    member: Option<u64>,
) -> std::io::Result<()> {
    let times = trace.times();
    let columns: Vec<&[f64]> = names.iter().filter_map(|n| trace.signal(n)).collect();
    for (row, &time) in times.iter().enumerate() {
        let mut line = String::new();
        if let Some(member) = member {
            line.push_str(&member.to_string());
            line.push(',');
        }
        line.push_str(&time.to_string());
        for column in &columns {
            line.push(',');
            line.push_str(&column[row].to_string());
        }
        writeln!(stdout, "{line}")?;
    }
    Ok(())
}