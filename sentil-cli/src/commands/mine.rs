//! `sentil mine` finds the tightest value of one spec parameter for which the specification still holds over a trace

use sentil::{mine_tightest_parameter, Formula, SpecRegistry};
use serde_json::json;

use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::Out;

#[allow(clippy::too_many_arguments)]
pub fn run(
    spec: &str,
    variant: Option<&str>,
    params: &[String],
    parameter: &str,
    range: Option<&str>,
    trace_path: &str,
    map: &[String],
    out: &Out,
) -> Run {
    let probe = engine::resolve_builder(spec, variant, params)?;
    let available = probe.parameters();
    if !available.contains_key(parameter) {
        let mut names: Vec<&str> = available.keys().map(String::as_str).collect();
        names.sort_unstable();
        return Err(CliError::Input(
            format!("spec '{spec}' has no parameter '{parameter}'"),
            Some(format!("parameters: {}", names.join(", "))),
        ));
    }
    let declared = probe.template().parameters.get(parameter).and_then(|d| d.range);
    let (lower, upper) = match range {
        Some(text) => {
            let (mut lo, mut hi) = parse_range(text)?;
            if let Some([dlo, dhi]) = declared {
                lo = lo.max(dlo);
                hi = hi.min(dhi);
                if lo > hi {
                    return Err(CliError::Input(
                        format!("--range lies outside '{parameter}'s allowed [{dlo}, {dhi}]"),
                        None,
                    ));
                }
            }
            (lo, hi)
        }
        None => declared.map(|[lo, hi]| (lo, hi)).ok_or_else(|| {
            CliError::Input(
                format!("spec '{spec}' defines no range for '{parameter}'"),
                Some("give one with --range lo,hi".into()),
            )
        })?,
    };

    let overrides = engine::parse_params(params)?;
    let trace = engine::load_trace(trace_path, map)?;
    let make = |value: f64| -> sentil::Result<Formula> {
        let mut builder = SpecRegistry::global().builder(spec)?;
        if let Some(variant) = variant {
            builder = builder.with_variant(variant)?;
        }
        for (key, set) in &overrides {
            builder = builder.with_param(key, *set)?;
        }
        builder.with_param(parameter, value)?.build_formula()
    };

    let tightest = mine_tightest_parameter(make, std::slice::from_ref(&trace), lower, upper)
        .map_err(|e| CliError::Engine(e.to_string()))?;

    if out.is_text() {
        out.heading("mine");
        out.field("spec", spec);
        out.field("parameter", parameter);
        out.field("range", &format!("[{lower}, {upper}]"));
        out.field("tightest", &format!("{tightest:.6}"));
    } else {
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "mine",
                "spec": spec,
                "parameter": parameter,
                "range": [lower, upper],
                "tightest": tightest,
            })
        );
    }
    Ok(code::SUCCESS)
}

fn parse_range(text: &str) -> Result<(f64, f64), CliError> {
    let (lo, hi) = text.split_once(',').ok_or_else(|| {
        CliError::Input(format!("--range '{text}' must be lo,hi"), Some("for example --range 0,1".into()))
    })?;
    let lower = lo.trim().parse().map_err(|_| {
        CliError::Input(format!("--range lower bound '{lo}' is not a number"), None)
    })?;
    let upper = hi.trim().parse().map_err(|_| {
        CliError::Input(format!("--range upper bound '{hi}' is not a number"), None)
    })?;
    Ok((lower, upper))
}