//! `sentil monitor`: the online monitor. One JSON sample per line in, one verdict
//! per line out, no whole-trace buffering, so it sits in a pipe between a sensor
//! and an alerter.

use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sentil::{MultiFormulaMonitor, Robustness};
use serde_json::json;

use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

/// Builds the bank, then folds each line of input into it until the stream ends or
/// the user interrupts, in which case it drains and exits 130.
pub fn run(
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    out: &Out,
) -> Run {
    let (combined, _builder) = engine::resolve_formula(formula, spec, variant, params, false)?;
    let formulas: Vec<&str> = combined
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if formulas.is_empty() {
        return Err(CliError::Input("there is no formula to monitor".into(), None));
    }

    let mut monitor = MultiFormulaMonitor::new();
    let ids: Vec<String> = (0..formulas.len()).map(|i| format!("f{i}")).collect();
    for (id, text) in ids.iter().zip(&formulas) {
        let parsed = engine::parse_or_diagnose(text)?;
        monitor
            .add_formula(id.clone(), &parsed)
            .map_err(|e| CliError::Engine(e.to_string()))?;
    }

    let interrupted = Arc::new(AtomicBool::new(false));
    let flag = interrupted.clone();
    let _ = ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst));

    if out.is_text() && !out.quiet {
        let banner = format!(
            "monitoring {} formula(s); send one JSON object per line, e.g. {{\"time\": 1.0, \"x\": 5.0}}",
            formulas.len()
        );
        eprintln!("{}", out.paint(&banner, output::dim()));
    }

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut samples = 0u64;
    for line in stdin.lock().lines() {
        if interrupted.load(Ordering::SeqCst) {
            break;
        }
        let line = line.map_err(|e| CliError::Input(format!("reading input: {e}"), None))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (time, pairs) = parse_sample(trimmed)?;
        let borrowed: Vec<(&str, f64)> = pairs.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let results = monitor
            .update(time, &borrowed)
            .map_err(|e| CliError::Engine(e.to_string()))?;
        emit(&mut stdout, out, time, &results)?;
        samples += 1;
    }

    if out.is_ndjson() {
        let _ = writeln!(
            stdout,
            "{}",
            json!({ "schema_version": "1.0", "event": "summary", "samples": samples })
        );
    }

    Ok(if interrupted.load(Ordering::SeqCst) {
        code::INTERRUPTED
    } else {
        code::SUCCESS
    })
}

fn parse_sample(line: &str) -> Result<(f64, Vec<(String, f64)>), CliError> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| CliError::Input(format!("invalid JSON sample: {e}"), None))?;
    let object = value
        .as_object()
        .ok_or_else(|| CliError::Input("each sample must be a JSON object".into(), None))?;
    let time = object
        .get("time")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| CliError::Input("the sample has no numeric 'time'".into(), None))?;
    let mut pairs = Vec::new();
    for (key, raw) in object {
        if key == "time" {
            continue;
        }
        if let Some(number) = raw.as_f64() {
            pairs.push((key.clone(), number));
        }
    }
    Ok((time, pairs))
}

/// Writes one verdict record for a sample.
fn emit(
    stdout: &mut std::io::Stdout,
    out: &Out,
    time: f64,
    results: &[(String, Robustness)],
) -> Result<(), CliError> {
    let write_result = if out.is_text() {
        let mut line = format!("[t={time:.3}]");
        for (id, robustness) in results {
            let value = robustness.value();
            let verdict = if value >= 0.0 {
                out.paint("sat", output::good())
            } else {
                out.paint("viol", output::bad())
            };
            let provisional = if robustness.is_resolved() { "" } else { "~" };
            line.push_str(&format!("  {id} {verdict} {provisional}{value:.4}"));
        }
        writeln!(stdout, "{line}")
    } else {
        let mut map = serde_json::Map::new();
        for (id, robustness) in results {
            map.insert(
                id.clone(),
                json!({
                    "robustness": robustness.value(),
                    "resolved": robustness.is_resolved(),
                }),
            );
        }
        writeln!(
            stdout,
            "{}",
            json!({ "schema_version": "1.0", "event": "sample", "time": time, "results": map })
        )
    };
    // A closed downstream pipe is a normal end to a stream, not an error to report.
    match write_result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(CliError::Internal(format!("writing output: {e}"))),
    }
}