//! GPU batching for the CMA-ES synthesis search.

use super::cmaes::{cma_es_batched, CmaConfig};
use super::model::{Bounds, SystemModel};
use super::smooth::{SmoothConfig, SoftKind};
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::gpu::{build_soft_forward_shader, SynthForwardContext};
use crate::signal::Trace;

const GPU_MIN_POPULATION: usize = 32;

/// The CMA-ES population for `dimension`, mirroring `cma_es`'s default rule.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the input dimension is small and exact in f64"
)]
fn population(dimension: usize, configured: usize) -> usize {
    if configured == 0 {
        4 + (3.0 * (dimension.max(1) as f64).ln()).floor() as usize
    } else {
        configured
    }
}

/// The time grid as f32, or `None` when a timestamp is not exactly representable in
/// f32.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the device evaluates in f32 by contract"
)]
#[allow(
    clippy::float_cmp,
    reason = "the exact round-trip equality is the representability test itself"
)]
fn times_f32(trace: &Trace) -> Option<Vec<f32>> {
    trace
        .times()
        .iter()
        .map(|&t| {
            let f = t as f32;
            (f64::from(f) == t).then_some(f)
        })
        .collect()
}

/// Runs the CMA-ES search with the population scored on the GPU, or returns `None`
/// to fall back to the CPU when the problem is ineligible or no device is present.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the device evaluates in f32 by contract"
)]
pub(crate) fn solve_cmaes_gpu<M: SystemModel>(
    model: &M,
    spec: &Formula,
    smooth: SmoothConfig,
    bounds: &Bounds,
    start: &[f64],
    config: CmaConfig,
) -> Option<Result<Vec<f64>>> {
    if smooth.kind() != SoftKind::LogSumExp
        || population(start.len(), config.population) < GPU_MIN_POPULATION
    {
        return None;
    }
    let symbols = spec.variables();
    if symbols.is_empty() {
        return None;
    }
    let initial = model.initial_state();
    let probe = model.rollout_from(initial, start).ok()?;
    let times = times_f32(&probe)?;
    let trace_len = times.len();
    let (shader, _) =
        build_soft_forward_shader(spec, &symbols, trace_len, smooth.temperature()).ok()?;
    let context = SynthForwardContext::new(&shader).ok()?;

    let variables = symbols.len();
    let objective = |inputs: &[Vec<f64>]| -> Result<Vec<f64>> {
        let mut packed = Vec::with_capacity(inputs.len() * variables * trace_len);
        for input in inputs {
            let trace = model.rollout_from(initial, input)?;
            for name in &symbols {
                let column = trace
                    .signals()
                    .get(name)
                    .ok_or_else(|| Error::UnknownVariable { name: name.clone() })?;
                packed.extend(column.iter().map(|&x| x as f32));
            }
        }
        let scores = context.score_batch(&packed, &times, inputs.len())?;
        Ok(scores.into_iter().map(f64::from).collect())
    };
    Some(cma_es_batched(objective, start, bounds, config).map(|(input, _)| input))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::cast_possible_truncation,
        reason = "the device evaluates in f32; the test signals are small"
    )]
    #![allow(
        clippy::type_complexity,
        reason = "the device test carries a literal case table"
    )]

    use super::*;
    use crate::synthesis::{Backend, LinearModel, SynthesisProblem, Synthesizer};

    fn gpu_soft(formula: &Formula, trace: &Trace, beta: f64) -> f32 {
        let symbols = formula.variables();
        let l = trace.times().len();
        let (shader, _) = build_soft_forward_shader(formula, &symbols, l, beta).unwrap();
        let context = SynthForwardContext::new(&shader).unwrap();
        let mut packed = Vec::new();
        for name in &symbols {
            let column = trace.signals().get(name).unwrap();
            packed.extend(column.iter().map(|&x| x as f32));
        }
        let times: Vec<f32> = trace.times().iter().map(|&t| t as f32).collect();
        context.score_batch(&packed, &times, 1).unwrap()[0]
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn gpu_soft_robustness_matches_the_cpu_within_f32_tolerance() {
        let cases: &[(&str, &[f64], &[(&str, &[f64])])] = &[
            (
                "always[0, 3](x > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 0.5, 3.0])],
            ),
            (
                "eventually[0, 3](x > 2)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 3.0, 0.0])],
            ),
            (
                "always[0, 3](x > 0 and y > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 1.0, 3.0]), ("y", &[2.0, 1.0, 3.0, 1.0])],
            ),
            (
                "(x > 0) until[0, 3] (y > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 1.0, 1.0, 1.0]), ("y", &[-1.0, -1.0, 2.0, 1.0])],
            ),
            (
                "always[0, 3](eventually[0, 1](x > 1))",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[2.0, 0.0, 2.0, 0.0])],
            ),
        ];
        for (formula_str, times, signals) in cases {
            let formula = Formula::parse(formula_str).unwrap();
            let mut trace = Trace::new(times.to_vec()).unwrap();
            for (name, values) in *signals {
                trace.add_signal(name, values.to_vec()).unwrap();
            }
            for &beta in &[2.0, 10.0, 50.0] {
                let config = SmoothConfig::new(beta).unwrap();
                let cpu = formula.smooth_robustness(&trace, config).unwrap();
                let gpu = f64::from(gpu_soft(&formula, &trace, beta));
                let tol = 1e-3_f64.max(1e-3 * cpu.abs());
                assert!(
                    (gpu - cpu).abs() < tol,
                    "`{formula_str}` at beta {beta}: gpu {gpu} vs cpu {cpu}"
                );
            }
        }
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn a_gpu_scored_search_satisfies_a_reachable_spec() {
        let model =
            LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["pos"], 1.0, 5).unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_backend(Backend::CmaEs)
            .with_population(48)
            .with_budget(400)
            .on_gpu(true);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(result.satisfies, "robustness {}", result.robustness);
    }
}