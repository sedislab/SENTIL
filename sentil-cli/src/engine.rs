use std::io::Read;
use std::path::Path;

use sentil::{Formula, SpecBuilder, SpecRegistry, Trace};

use crate::error::{span_at, CliError};

/// Resolves the user's `--formula`/`--spec` choice into a formula string, plus the builder when a spec was used, since `smc` takes its noise models from it.
pub fn resolve_formula(
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    probabilistic: bool,
) -> Result<(String, Option<SpecBuilder>), CliError> {
    match (formula, spec) {
        (Some(text), None) => Ok((text.to_string(), None)),
        (None, Some(name)) => {
            let mut builder = SpecRegistry::global().builder(name).map_err(|e| {
                CliError::Input(
                    format!("cannot load spec '{name}': {e}"),
                    Some("run `sentil specs` to list the available specifications".into()),
                )
            })?;
            if let Some(variant) = variant {
                builder = builder
                    .with_variant(variant)
                    .map_err(|e| CliError::Input(format!("variant '{variant}': {e}"), None))?;
            }
            for param in params {
                let (key, value) = parse_param(param)?;
                builder = builder
                    .with_param(&key, value)
                    .map_err(|e| CliError::Input(format!("parameter '{key}': {e}"), None))?;
            }
            let formula = if probabilistic {
                builder.build_probabilistic().map_err(|e| {
                    CliError::Input(format!("building the probabilistic formula: {e}"), None)
                })?
            } else {
                builder
                    .build_deterministic()
                    .map_err(|e| CliError::Input(format!("building the formula: {e}"), None))?
            };
            Ok((formula, Some(builder)))
        }
        (Some(_), Some(_)) => Err(CliError::Input(
            "give either --formula or --spec, not both".into(),
            None,
        )),
        (None, None) => Err(CliError::Input(
            "give a formula with -f or a specification with --spec".into(),
            Some("for example -f 'always[0,5] (x > 0)' or --spec controls/overshoot".into()),
        )),
    }
}

fn parse_param(text: &str) -> Result<(String, f64), CliError> {
    let (key, value) = text.split_once('=').ok_or_else(|| {
        CliError::Input(
            format!("parameter '{text}' must be key=value"),
            Some("for example -p limit=1.5".into()),
        )
    })?;
    let parsed = value.trim().parse().map_err(|_| {
        CliError::Input(format!("parameter '{key}' value '{value}' is not a number"), None)
    })?;
    Ok((key.trim().to_string(), parsed))
}

pub fn parse_or_diagnose(formula: &str) -> Result<Formula, CliError> {
    match Formula::parse(formula) {
        Ok(parsed) => Ok(parsed),
        Err(sentil::Error::Parse(e)) => Err(CliError::Parse {
            src: formula.to_string(),
            span: span_at(formula, e.line, e.column),
            label: e.message.clone(),
            help: None,
        }),
        Err(e) => Err(CliError::Engine(e.to_string())),
    }
}

/// Reads a trace from `path`, or from standard input when `path` is `-`. A named
/// file that does not exist is reported as missing rather than as a read error.
pub fn load_trace(path: &str) -> Result<Trace, CliError> {
    if path == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| CliError::Input(format!("reading standard input: {e}"), None))?;
        return trace_from_text(&text);
    }
    if !Path::new(path).exists() {
        return Err(CliError::NotFound {
            path: path.to_string(),
        });
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| CliError::Input(format!("{path}: {e}"), None))?;
    trace_from_text(&text)
}

fn trace_from_text(text: &str) -> Result<Trace, CliError> {
    match text.trim_start().chars().next() {
        Some('[') => trace_from_json(text),
        _ => trace_from_csv(text.as_bytes()),
    }
}

fn trace_from_csv(bytes: &[u8]) -> Result<Trace, CliError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|e| CliError::Input(format!("reading the CSV header: {e}"), None))?
        .clone();
    if headers.len() < 2 {
        return Err(CliError::Input(
            "the trace needs a time column and at least one signal column".into(),
            None,
        ));
    }
    let names: Vec<String> = headers.iter().skip(1).map(str::to_string).collect();
    let mut times = Vec::new();
    let mut columns: Vec<Vec<f64>> = vec![Vec::new(); names.len()];
    for (row, record) in reader.records().enumerate() {
        let record = record.map_err(|e| CliError::Input(format!("row {}: {e}", row + 1), None))?;
        times.push(cell(&record, 0, "time", row)?);
        for (i, name) in names.iter().enumerate() {
            columns[i].push(cell(&record, i + 1, name, row)?);
        }
    }
    build_trace(names, times, columns)
}

fn cell(record: &csv::StringRecord, index: usize, name: &str, row: usize) -> Result<f64, CliError> {
    let raw = record.get(index).unwrap_or("").trim();
    raw.parse().map_err(|_| {
        CliError::Input(format!("row {}: {name} value '{raw}' is not a number", row + 1), None)
    })
}

fn trace_from_json(text: &str) -> Result<Trace, CliError> {
    let records: Vec<serde_json::Value> = serde_json::from_str(text).map_err(|e| {
        CliError::Input(
            format!("invalid JSON trace: {e}"),
            Some("expected an array of {\"time\": .., \"x\": ..} records".into()),
        )
    })?;
    let first = records.first().and_then(|r| r.as_object()).ok_or_else(|| {
        CliError::Input("the JSON trace is empty or not an array of objects".into(), None)
    })?;
    let mut names: Vec<String> = first.keys().filter(|k| *k != "time").cloned().collect();
    names.sort();
    let mut times = Vec::with_capacity(records.len());
    let mut columns: Vec<Vec<f64>> = vec![Vec::with_capacity(records.len()); names.len()];
    for (i, record) in records.iter().enumerate() {
        let object = record.as_object().ok_or_else(|| {
            CliError::Input(format!("record {} is not an object", i + 1), None)
        })?;
        let time = object.get("time").and_then(serde_json::Value::as_f64).ok_or_else(|| {
            CliError::Input(format!("record {} has no numeric 'time'", i + 1), None)
        })?;
        times.push(time);
        for (j, name) in names.iter().enumerate() {
            let value = object.get(name).and_then(serde_json::Value::as_f64).ok_or_else(|| {
                CliError::Input(format!("record {}: '{name}' is missing or not a number", i + 1), None)
            })?;
            columns[j].push(value);
        }
    }
    build_trace(names, times, columns)
}

fn build_trace(names: Vec<String>, times: Vec<f64>, columns: Vec<Vec<f64>>) -> Result<Trace, CliError> {
    let mut trace =
        Trace::new(times).map_err(|e| CliError::Input(format!("the trace times are invalid: {e}"), None))?;
    for (name, column) in names.into_iter().zip(columns) {
        trace
            .add_signal(&name, column)
            .map_err(|e| CliError::Input(format!("signal '{name}': {e}"), None))?;
    }
    Ok(trace)
}