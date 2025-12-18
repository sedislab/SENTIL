use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use sentil::{Formula, LiftingRegistry, NoiseInteraction, NoiseModel, SpecBuilder, SpecRegistry, Trace};

use crate::error::{span_at, CliError};

/// Builds a lifting registry from `--noise` flags, each `signal=distribution:params` with an optional `:additive` or `:multiplicative` tail.
pub fn parse_noise(specs: &[String]) -> Result<Option<LiftingRegistry>, CliError> {
    if specs.is_empty() {
        return Ok(None);
    }
    let mut registry = LiftingRegistry::new();
    for spec in specs {
        let (signal, rest) = spec.split_once('=').ok_or_else(|| {
            CliError::Input(
                format!("--noise '{spec}' must be signal=distribution:params"),
                Some("for example --noise 'speed=gaussian:0,0.5'".into()),
            )
        })?;
        let parts: Vec<&str> = rest.split(':').collect();
        let params = if parts.len() > 1 && !parts[1].is_empty() {
            parts[1]
                .split(',')
                .map(|p| p.trim().parse::<f64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    CliError::Input(format!("--noise '{spec}': parameters must be numbers"), None)
                })?
        } else {
            Vec::new()
        };
        let interaction = match parts.get(2).map(|s| s.to_ascii_lowercase()).as_deref() {
            None | Some("additive") => NoiseInteraction::Additive,
            Some("multiplicative") => NoiseInteraction::Multiplicative,
            Some(other) => {
                return Err(CliError::Input(
                    format!("--noise '{spec}': interaction must be additive or multiplicative, not '{other}'"),
                    None,
                ))
            }
        };
        let model = noise_model(parts[0], &params)
            .map_err(|e| CliError::Input(format!("--noise '{spec}': {e}"), None))?;
        registry.register(signal.trim(), model, interaction);
    }
    Ok(Some(registry))
}

fn noise_model(distribution: &str, params: &[f64]) -> Result<NoiseModel, String> {
    let need = |n: usize| -> Result<(), String> {
        if params.len() == n {
            Ok(())
        } else {
            Err(format!("{distribution} needs {n} parameter(s), got {}", params.len()))
        }
    };
    let m = |r: Result<NoiseModel, sentil::Error>| r.map_err(|e| e.to_string());
    match distribution.to_ascii_lowercase().as_str() {
        "gaussian" | "normal" => need(2).and_then(|()| m(NoiseModel::gaussian(params[0], params[1]))),
        "uniform" => need(2).and_then(|()| m(NoiseModel::uniform(params[0], params[1]))),
        "lognormal" | "log_normal" => need(2).and_then(|()| m(NoiseModel::log_normal(params[0], params[1]))),
        "exponential" | "exp" => need(1).and_then(|()| m(NoiseModel::exponential(params[0]))),
        "gamma" => need(2).and_then(|()| m(NoiseModel::gamma(params[0], params[1]))),
        "beta" => need(2).and_then(|()| m(NoiseModel::beta(params[0], params[1]))),
        "dirac" | "constant" => need(1).and_then(|()| m(NoiseModel::dirac(params[0]))),
        "weibull" => need(2).and_then(|()| m(NoiseModel::weibull(params[0], params[1]))),
        "rayleigh" => need(1).and_then(|()| m(NoiseModel::rayleigh(params[0]))),
        "gumbel" => need(2).and_then(|()| m(NoiseModel::gumbel(params[0], params[1]))),
        "cauchy" => need(2).and_then(|()| m(NoiseModel::cauchy(params[0], params[1]))),
        "student_t" | "studentt" => need(3).and_then(|()| m(NoiseModel::student_t(params[0], params[1], params[2]))),
        "truncated_normal" | "truncnormal" => {
            need(4).and_then(|()| m(NoiseModel::truncated_normal(params[0], params[1], params[2], params[3])))
        }
        "poisson" => need(1).and_then(|()| m(NoiseModel::poisson(params[0]))),
        "binomial" => need(2).and_then(|()| m(NoiseModel::binomial(params[0] as u64, params[1]))),
        other => Err(format!(
            "unknown distribution '{other}'; try gaussian, uniform, lognormal, exponential, gamma, \
             beta, dirac, weibull, rayleigh, gumbel, cauchy, student_t, truncated_normal, poisson, \
             or binomial"
        )),
    }
}

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
            let builder = resolve_builder(name, variant, params)?;
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

pub fn resolve_builder(
    name: &str,
    variant: Option<&str>,
    params: &[String],
) -> Result<SpecBuilder, CliError> {
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
    Ok(builder)
}

/// Parses the `-p key=value` overrides.
pub fn parse_params(specs: &[String]) -> Result<Vec<(String, f64)>, CliError> {
    specs.iter().map(|s| parse_param(s)).collect()
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

/// Reads a trace from `path`, or from standard input when `path` is `-`, then binds the formula's variables to dataset columns per `map` (each `variable=column`).
/// File type is inferred from the content or file extension.
pub fn load_trace(path: &str, map: &[String]) -> Result<Trace, CliError> {
    let parsed = parse_map(map)?;
    let trace = read_trace(path)?;
    remap_trace(trace, &parsed)
}

fn read_trace(path: &str) -> Result<Trace, CliError> {
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
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "" | "csv" | "tsv" | "txt" | "json" | "ndjson" => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| CliError::Input(format!("{path}: {e}"), None))?;
            trace_from_text(&text)
        }
        _ => Trace::from_path(path).map_err(|e| {
            CliError::Input(
                format!("{path}: {e}"),
                Some("Parquet, Arrow, and SQLite need a build with --features formats".into()),
            )
        }),
    }
}

/// Parses the `--map variable=column` pairs.
pub fn parse_map(specs: &[String]) -> Result<Vec<(String, String)>, CliError> {
    specs
        .iter()
        .map(|spec| {
            let (variable, column) = spec.split_once('=').ok_or_else(|| {
                CliError::Input(
                    format!("--map '{spec}' must be variable=column"),
                    Some("for example --map speed=velocity_mps".into()),
                )
            })?;
            Ok((variable.trim().to_string(), column.trim().to_string()))
        })
        .collect()
}

fn remap_trace(trace: Trace, map: &[(String, String)]) -> Result<Trace, CliError> {
    if map.is_empty() {
        return Ok(trace);
    }
    let columns = trace.variables();
    for (variable, column) in map {
        if !columns.iter().any(|c| c == column) {
            return Err(CliError::Input(
                format!("--map {variable}={column}: no column '{column}' in the trace"),
                Some(format!("the trace has: {}", columns.join(", "))),
            ));
        }
    }
    let rename: HashMap<&str, &str> = map.iter().map(|(v, c)| (c.as_str(), v.as_str())).collect();
    let mut out = Trace::new(trace.times().to_vec())
        .map_err(|e| CliError::Input(format!("the trace times are invalid: {e}"), None))?;
    for name in trace.variables() {
        let target = rename.get(name).copied().unwrap_or(name);
        let values = trace.signal(name).unwrap_or(&[]).to_vec();
        out.add_signal(target, values)
            .map_err(|e| CliError::Input(format!("signal '{target}': {e}"), None))?;
    }
    Ok(out)
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
        if let Some(extra) = object.keys().find(|k| *k != "time" && !names.iter().any(|n| n == *k)) {
            return Err(CliError::Input(
                format!("record {}: unexpected signal '{extra}'", i + 1),
                Some("every record must carry the same signals as the first".into()),
            ));
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